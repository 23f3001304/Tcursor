# src-tauri/src/export/preview/mod.rs

Single-frame preview engine + the warm renderer cache. Renders one composited output frame at an arbitrary scrub time T from `edit.json`, reusing `FrameRenderer` (render.rs) so the preview is byte-faithful to the export, and exposes the export background as an image. The lightweight metadata commands the editor uses for smooth playback (camera curve, layout, clicks, proxy) live in `preview_track.rs`; both share the `with_warm` cache helper here.

## render_preview

```rust
pub fn render_preview(paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>>
```

Renders one composited frame at `time_ms` (uncached: builds a fresh renderer via `build_renderer`) and returns PNG bytes at the resolved preview size. Not currently called by any Tauri command (`preview_frame`, below, uses the warm cache instead) - kept as the uncached entry point.

### Inputs (what, and why it is needed)

- `paths: &ProjectPaths` - project folder. *Why:* all asset paths (video, webcam, events, edit.json) are derived from it.
- `time_ms: u32` - preview scrub position in ms from the start of the recording. *Why:* the caller knows the editor timeline position; the function converts it to an output frame index internally.

### Returns

`Result<Vec<u8>>` - PNG bytes of the composited frame, sized to the resolved preview canvas (`PREVIEW_LONG_EDGE`, following the doc's aspect). Errors if `FrameRenderer::new` fails (missing files), if the screen decoder cannot be spawned, if `time_ms` is past the end of the video, or if in-process PNG encoding (`png_encode`) fails.

### Implementation

1. `build_renderer(paths)` calls `FrameRenderer::new(paths, Layout::default(), fps, Resolution::Source, Some(PREVIEW_LONG_EDGE))` - the doc's `aspect` is resolved against the true source dims then downscaled to the `PREVIEW_LONG_EDGE` (1280px) budget (`Layout::resolve`), so the preview frame always matches the export's aspect proportionally. `Resolution::Source` is a no-op here (the export resolution setting only applies to the export build).
2. Compute `k_target = time_ms as u64 * OUT_FPS / 1000`. Fast-forward the camera sim by calling `step_camera` for every `j` in `0..=k_target` at `video_start + j * 1000 / OUT_FPS`. This must ascend because `CameraSim` and the cursor index only move forward. The last returned `FramePose` is the preview pose. Cost: arithmetic only, no I/O.
3. Spawn a `RawDecoder` on `paths.video()` seeked to `time_ms` (the screen file's frame 0 is `video_start`, so `time_ms` is the right offset), read one frame into a `screen_bytes`-sized buffer; bail if the read hits EOF (time past end of video).
4. If `paths.webcam().exists()`, spawn a `RawDecoder` on the webcam seeked to `video_start + time_ms` (export pre-seeks the webcam by `video_start`, so its file-time is shifted) with `cover_scale = Some((webcam_w, webcam_h))` - the same panel-aspect decode box the export uses, so the preview and the export never disagree about the webcam's shape - read one frame; else `webcam = None`.
5. Call `renderer.composite_at(&pose, &screen_buf, webcam_ref, &mut bgra)` to write a BGRA buffer into `bgra`.
6. Call `png_encode(bgra, meta.out_w, meta.out_h)` to produce PNG bytes in-process via the `png` crate (no ffmpeg subprocess involved), at the renderer's resolved size.

## preview_frame

```rust
#[tauri::command]
pub async fn preview_frame(folder: String, time_ms: u32, app: tauri::AppHandle) -> Result<String, String>
```

Tauri IPC command: renders one preview frame (via the warm cache) and returns a PNG data URL.

**Off the main thread (Task 41 sweep correction).** `async fn` + `spawn_blocking`, same freeze mechanism as `ai::commands` (Task 40) and `thumbs.rs`/`preview_track.rs` (Task 41): `render_frame` calls `RawDecoder::spawn` (screen, and webcam when present) which shells out to `ffmpeg` and blocks on its stdout pipe until the seeked frame decodes - a real blocking subprocess call, not in-process math. An earlier T41 sweep incorrectly grouped this command with `camera_track`/`preview_layout`/`click_track` (`preview_track.rs`) as "pure math/cache reads" and left it sync - those three genuinely are pure math (no decode, see their own docs); this one is not. Because `session: tauri::State<'_, PreviewSession>` can't be moved into `spawn_blocking` (its lifetime isn't `'static`), the command instead takes `app: tauri::AppHandle` (`'static`, `Clone`, `Send`) and re-derives the same managed-state handle inside the blocking closure via `app.state::<PreviewSession>()` (`tauri::Manager`).

**Verified uncalled by the frontend today** (Task 41 sweep correction - grepped `src/` for `preview_frame`/`previewFrame`, no hits) - the editor's M3 preview plays the recording natively via `<video>` (see `Editor.md`) rather than fetching per-frame PNGs, so nothing currently invokes this command. Converted anyway per "dead-or-not, it must not be a landmine" - a future caller (or a re-enabled `render_preview`-style flow) would otherwise silently reintroduce a main-thread freeze.

### Inputs (what, and why it is needed)

- `folder: String` - absolute path to the project directory. *Why:* the frontend holds the folder path from `stopRecording`; it is the stable identity for a recording session across Tauri calls.
- `time_ms: u32` - scrub position in ms. *Why:* the editor timeline drives this; the frontend passes the current playhead position.
- `app: tauri::AppHandle` - resolves the `PreviewSession` managed state from inside the `spawn_blocking` closure (see above).

### Returns

`Result<String, String>` - on success, a `data:image/png;base64,...` data URL ready for use in an `<img>` `src` attribute. On error, a human-readable error string that the frontend can display. A `spawn_blocking` join failure also maps to `Err(String)`, same shape as every other failure this command can return.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking(move || { ... })`, `.await`ed then `?`-unwrapped:

1. `app.state::<PreviewSession>()` re-derives the managed-state handle.
2. Via `with_warm`, call `render_frame` at `time_ms` and the cached size.
3. Base64-encode the PNG bytes with the local `base64_encode` helper (RFC 4648 alphabet, no line breaks).
4. Prefix with `"data:image/png;base64,"` and return.

## preview_bg

```rust
#[tauri::command]
pub fn preview_bg(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<String, String>
```

Returns the export background (the BGRA mesh/gradient the compositor draws under the screen) as a PNG data URL, so the editor's canvas preview paints the exact same background the export uses instead of an approximate gradient.

**Stays sync (verified, Task 41 sweep correction).** On a warm cache (the common case - the background doesn't change per-edit, so the frontend fetches it once per folder and it stays cached in `PreviewSession`) this command's own call chain is `with_warm`'s fast path + `accessors::bg` (`&self.bg`, a plain field read, no I/O) + `png_encode` (in-memory only, the `png` crate, no subprocess) - verified by reading both implementations. The COLD-build path (`with_warm` rebuilding a renderer from scratch, e.g. first open or an aspect change) can still shell out to ffmpeg internally via `FrameRenderer::new`'s background decode - but that warm-up cost is shared unconditionally by every `PreviewSession`-based command (`camera_track`/`preview_layout`/`click_track` included), all of which stay off `spawn_blocking` today for the same `tauri::State` lifetime reason - a pre-existing, broader seam this fix doesn't attempt to close.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `session: State<PreviewSession>` - the warm renderer cache. *Why:* the background is decoded once with the renderer; reusing the cache avoids rebuilding it.

### Returns

`Result<String, String>` - a `data:image/png;base64,...` URL of the background. Errors (as a string) if the renderer cannot be built or the PNG encode fails.

### Implementation

1. Via `with_warm`, PNG-encode `renderer.bg()` at the cached `out_w` x `out_h`.
2. Base64-encode and prefix with `"data:image/png;base64,"`.

## with_warm

```rust
pub(crate) fn with_warm<T>(session: &PreviewSession, folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>) -> Result<T, String>
```

Runs `f` with the warm `FrameRenderer` for `folder`. A full rebuild happens only when the folder or the doc's `aspect` changes (both resize the frame, so the cached GPU compositor/background/FX - sized for the OLD dims - cannot just refresh); when just `edit.json`'s mtime changed with the SAME aspect (a zoom/spotlight edit) it calls `FrameRenderer::reload_edit` instead - refreshing the cheap edit-derived state in place while keeping the GPU device, FX, and background, so editing stays snappy (a full rebuild is ~seconds; the in-place refresh is ~microseconds). The single place the preview cache is keyed - shared by every preview command (`preview_frame`, `preview_bg`, and the `preview_track.rs` commands `camera_track` / `preview_layout` / `click_track`) so the warm-up logic lives exactly once. The closure receives the cached renderer + the resolved `ProjectPaths`; its return value is owned (the mutex guard is dropped on return).

### Implementation

Checks `folder` + `edit.json` mtime first (cheapest: one `metadata()` syscall, no JSON parse) - if both match the cached entry, reuses it as-is with no further work. Only when something changed does it peek `edit::seed::load_or_seed(&paths).aspect` (one small JSON read) to decide `reload_edit` (same aspect) vs a full `build_renderer` (different aspect, different folder, or first build for this folder).

## PreviewSession::has_webcam

```rust
pub fn has_webcam(&self) -> bool
```

Whether the CURRENTLY-CACHED preview renderer's project has a recorded webcam - used by `preview_fx_overlay` (`preview_fx.rs`) to gate the spotlight's camera-exclusion hole the same way the export gates it (`has_webcam` in `fx_state.rs`/`render/mod.rs`). Reads whichever renderer is warm right now via `FrameRenderer::has_webcam` (`render/accessors.rs`), regardless of `folder` - same as every other preview command implicitly relies on: the editor keeps at most one project's renderer warm via `with_warm`, refreshed by `camera_track`/`preview_layout`/etc. on essentially every render, so by the time an FX overlay is requested the warm renderer already belongs to the open project. Returns `false` (no hole) before anything has warmed the cache yet - fails safe, never an un-dimmed rectangle.

## PREVIEW_LONG_EDGE

```rust
const PREVIEW_LONG_EDGE: u32 = 1280;
```

The preview canvas long-edge budget in pixels. 1280 matches the old hardcoded 16:9 preview canvas (1280x720) exactly, so a default `Source` aspect on a 16:9 recording composites at an unchanged size; other aspects (or a non-16:9 source under `Source`) get a proportionally-sized canvas instead.

## build_renderer

```rust
fn build_renderer(paths: &ProjectPaths) -> Result<(FrameRenderer, RenderMeta)>
```

Builds a fresh preview renderer downscaled to `PREVIEW_LONG_EDGE`, following the doc's chosen `aspect` exactly via `FrameRenderer::new(paths, Layout::default(), fps, Resolution::Source, Some(PREVIEW_LONG_EDGE))` - the aspect is resolved against the true source dims then scaled to the long-edge budget, so the preview always matches the export's aspect proportionally. `Resolution::Source` is passed (rather than a user-chosen resolution) because the export resolution setting only applies to the export build, not the preview.
