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
4. If `paths.webcam().exists()`, spawn a `RawDecoder` on the webcam seeked to `video_start + time_ms` (export pre-seeks the webcam by `video_start`, so its file-time is shifted) with `scale = Some(webcam_size)`, read one frame; else `webcam = None`.
5. Call `renderer.composite_at(&pose, &screen_buf, webcam_ref, &mut bgra)` to write a BGRA buffer into `bgra`.
6. Call `png_encode(bgra, meta.out_w, meta.out_h)` to produce PNG bytes in-process via the `png` crate (no ffmpeg subprocess involved), at the renderer's resolved size.

## preview_frame

```rust
#[tauri::command]
pub fn preview_frame(folder: String, time_ms: u32, session: tauri::State<'_, PreviewSession>) -> Result<String, String>
```

Tauri IPC command: renders one preview frame (via the warm cache) and returns a PNG data URL.

### Inputs (what, and why it is needed)

- `folder: String` - absolute path to the project directory. *Why:* the frontend holds the folder path from `stopRecording`; it is the stable identity for a recording session across Tauri calls.
- `time_ms: u32` - scrub position in ms. *Why:* the editor timeline drives this; the frontend passes the current playhead position.
- `session: State<PreviewSession>` - the warm renderer cache. *Why:* scrubbing reuses one renderer instead of rebuilding per frame.

### Returns

`Result<String, String>` - on success, a `data:image/png;base64,...` data URL ready for use in an `<img>` `src` attribute. On error, a human-readable error string that the frontend can display.

### Implementation

1. Via `with_warm`, call `render_frame` at `time_ms` and the cached size.
2. Base64-encode the PNG bytes with the local `base64_encode` helper (RFC 4648 alphabet, no line breaks).
3. Prefix with `"data:image/png;base64,"` and return.

## preview_bg

```rust
#[tauri::command]
pub fn preview_bg(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<String, String>
```

Returns the export background (the BGRA mesh/gradient the compositor draws under the screen) as a PNG data URL, so the editor's canvas preview paints the exact same background the export uses instead of an approximate gradient.

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
