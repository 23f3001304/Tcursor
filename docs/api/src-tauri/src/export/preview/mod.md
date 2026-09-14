# src-tauri/src/export/preview/mod.rs

Single-frame preview engine + the warm renderer cache. Renders one composited output frame at an arbitrary scrub time T from `edit.json`, reusing `FrameRenderer` (render.rs) so the preview is byte-faithful to the export, and exposes the export background as an image. The lightweight metadata commands the editor uses for smooth playback (camera curve, layout, clicks, proxy) live in `preview_track.rs`; both share the `with_warm` cache helper (now in `session.rs`).

**The warm renderer cache moved (sweep-2 Task 1).** `Cached`, `PreviewSession` (with `has_webcam`) and `with_warm` now live in the sibling `session.rs` - see `session.md`. They are re-exported from here (`pub use session::PreviewSession;` / `pub(crate) use session::with_warm;`) so every preview command still imports them from `crate::export::preview`. They moved for the line budget and because the lock-discipline rewrite (build and render outside the cache mutex; `has_webcam` off the mutex entirely) wanted its own unit tests, which live in `session_tests.rs`.

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
2. Compute `k_target = time_ms as u64 * OUT_FPS / 1000`. Fast-forward the camera sim by calling `step_camera` for every `j` in `0..=k_target` at `video_start + j * 1000 / OUT_FPS`, each with `OUT_STEP_MS` as the frame period (the exact 16.667ms, never the rounded timestamp delta - see `render/mod.md`). This must ascend because `CameraSim` and the cursor index only move forward. The last returned `FramePose` is the preview pose. Cost: arithmetic only, no I/O.
3. Spawn a `RawDecoder` on `paths.video()` seeked to `time_ms` (the screen file's frame 0 is `video_start`, so `time_ms` is the right offset), read one frame into a `screen_bytes`-sized buffer; bail if the read hits EOF (time past end of video).
4. If `paths.webcam().exists()`, spawn a `RawDecoder` on the webcam seeked to `video_start + time_ms` (export pre-seeks the webcam by `video_start`, so its file-time is shifted) with `cover_scale = Some((webcam_w, webcam_h))` - the same SOURCE-aspect decode box the export uses (`render::meta::webcam_box`; each panel cover-crops it at composite time), so the preview and the export never disagree about the webcam's framing - read one frame; else `webcam = None`.

   **A webcam read failure here is deliberately NOT fatal to the frame.** Both EOF and a hard decode failure just leave the buffer as it was (and log a line): `webcam.webm` routinely ends before `video.mp4`, so an `-ss` past its end is an everyday scrub near the end of a clip, and failing the whole preview frame for it would make the last seconds of such a project un-scrubbable. This is the one place the honest-failure rule (`RawDecoder::classify_end`) is deliberately relaxed - a preview frame is not a deliverable, whereas the EXPORT surfaces the same failure as an `export-warning` and a screen failure as a hard `export-error`.
5. Call `renderer.composite_at(&pose, &screen_buf, webcam_ref, &mut bgra)` to write a BGRA buffer into `bgra`.
6. Call `png_encode(bgra, meta.out_w, meta.out_h)` to produce PNG bytes in-process via the `png` crate (no ffmpeg subprocess involved), at the renderer's resolved size.

## walk_to

```rust
pub(crate) fn walk_to(r: &mut FrameRenderer, video_start: u64, time_ms: u32) -> FramePose
```

Step the camera along the renderer's frame plan up to the output frame that shows clip time `time_ms` (`map.out_of(time_ms)` converted to an output frame index), through `FrameRenderer::walk_plan` with a no-op body, so the one-shot preview frame's camera is exactly the export's at that frame. With everything cut there is no plan and the camera is stepped once at output time 0 instead.

## preview_frame

```rust
#[tauri::command]
pub async fn preview_frame(folder: String, time_ms: u32, app: tauri::AppHandle) -> Result<String, String>
```

Tauri IPC command: renders one preview frame (via the warm cache) and returns a JPEG data URL - the export's own frame at that instant. **Why it exists (owner ruling 2026-09-14: the export is the reference):** the stage shows this frame whenever playback pauses or a scrub settles, so what the owner looks at while editing IS a frame of the export, whatever the live canvas approximated a moment earlier.

**Off the main thread (Task 41 sweep correction).** `async fn` + `spawn_blocking`, same freeze mechanism as `ai::commands` (Task 40) and `thumbs.rs`/`preview_track.rs` (Task 41): `render_frame` calls `RawDecoder::spawn` (screen, and webcam when present) which shells out to `ffmpeg` and blocks on its stdout pipe until the seeked frame decodes - a real blocking subprocess call, not in-process math. An earlier T41 sweep incorrectly grouped this command with `camera_track`/`preview_layout`/`click_track` (`preview_track.rs`) as "pure math/cache reads" and left it sync - those three genuinely are pure math (no decode, see their own docs); this one is not. (Sweep-2 Task 1 has since converted those three too - not because their own bodies decode, but because the `with_warm` cold path underneath every one of them does.) Because `session: tauri::State<'_, PreviewSession>` can't be moved into `spawn_blocking` (its lifetime isn't `'static`), the command instead takes `app: tauri::AppHandle` (`'static`, `Clone`, `Send`) and re-derives the same managed-state handle inside the blocking closure via `app.state::<PreviewSession>()` (`tauri::Manager`).

**Called by `useExactFrame`** (`src/editor/hooks/useExactFrame.ts`, via `ipcPreview.ts`'s `previewFrame`) whenever the stage's playhead rests; it was uncalled from the Task 41 sweep until 2026-09-14 - the editor's M3 preview plays the recording natively via `<video>` (see `Editor.md`) rather than fetching per-frame PNGs, so nothing currently invokes this command. Converted anyway per "dead-or-not, it must not be a landmine" - a future caller (or a re-enabled `render_preview`-style flow) would otherwise silently reintroduce a main-thread freeze.

### Inputs (what, and why it is needed)

- `folder: String` - absolute path to the project directory. *Why:* the frontend holds the folder path from `stopRecording`; it is the stable identity for a recording session across Tauri calls.
- `time_ms: u32` - scrub position in ms. *Why:* the editor timeline drives this; the frontend passes the current playhead position.
- `app: tauri::AppHandle` - resolves the `PreviewSession` managed state from inside the `spawn_blocking` closure (see above).

### Returns

`Result<String, String>` - on success, a `data:image/jpeg;base64,...` data URL ready for use in an `<img>` `src` attribute. On error, a human-readable error string that the frontend can display. A `spawn_blocking` join failure also maps to `Err(String)`, same shape as every other failure this command can return.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking(move || { ... })`, `.await`ed then `?`-unwrapped:

1. `app.state::<PreviewSession>()` re-derives the managed-state handle.
2. Via `with_warm`, call `composite_frame` at `time_ms` and the cached size, then `jpeg_encode`.
3. Base64-encode the JPEG bytes with the local `base64_encode` helper (RFC 4648 alphabet, no line breaks).
4. Prefix with `"data:image/jpeg;base64,"` and return.

## decode_screen

```rust
fn decode_screen(paths: &ProjectPaths, meta: &RenderMeta, time_ms: u32, buf: &mut [u8]) -> Result<()>
```

Seek-decodes one screen frame at `time_ms` (clip time) into `buf` as nv12 at the renderer's own crop. Factored out because `composite_frame` now decodes up to TWO frames: the one at `time_ms`, and - only when `FramePose::mix` says this instant is inside a mid-take display switch - the frame at `SpanMix::hold_ms`, the last output frame before the switch. That second one is the picture the EXPORT latches and dissolves from, so the ghost image in a mid-transition preview frame is the export's own. A failure on the second decode drops the dissolve rather than failing the scrub; only the export treats a screen decode as a deliverable.

## preview_bg

```rust
#[tauri::command]
pub async fn preview_bg(folder: String, app: tauri::AppHandle) -> Result<String, String>
```

Returns the export background (the BGRA mesh/gradient the compositor draws under the screen) as a PNG data URL, so the editor's canvas preview paints the exact same background the export uses instead of an approximate gradient.

**Now off the main thread (sweep-2 Task 1, superseding the Task 41 "stays sync" verdict).** The earlier verdict was right about the warm path - `with_warm`'s fast path + `accessors::bg` (a plain field read) + an in-memory `png_encode`, no subprocess - and wrong about what that path costs. Two things it did not weigh: the COLD path underneath it still runs `FrameRenderer::new` (ffprobe/ffmpeg subprocesses + wgpu init), which Task 41 explicitly deferred as "a broader seam this fix doesn't attempt to close" and which this task closes; and the "in-memory" encode is a full-size (1280x720 by default) BGRA->RGBA swizzle plus a zlib deflate plus a hand-rolled base64 of the result - several MB of work - which `useEditorData`'s `previewBg` effect re-runs on **every pointermove of every Background-panel slider**, because it is keyed on `JSON.stringify(doc.settings.background)`. `async fn` + `spawn_blocking`, taking `app: tauri::AppHandle` instead of `tauri::State<'_, PreviewSession>` for the usual lifetime reason (see `preview_frame`); the JS call is unchanged.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) from inside the `spawn_blocking` closure. *Why:* the background is decoded once with the renderer; reusing the cache avoids rebuilding it.

### Returns

`Result<String, String>` - a `data:image/png;base64,...` URL of the background. Errors (as a string) if the renderer cannot be built or the PNG encode fails; a `spawn_blocking` join failure maps to the same shape.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking`:

1. `app.state::<PreviewSession>()` re-derives the managed-state handle.
2. Via `with_warm`, PNG-encode `renderer.bg()` at the cached `out_w` x `out_h`.
3. Base64-encode and prefix with `"data:image/png;base64,"`.

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
