# src-tauri/src/export/preview/preview_track.rs

Editor-preview support commands: the lightweight metadata the M3 editor needs to play the recording natively and composite a smooth, export-faithful preview on a canvas - the per-frame camera curve, the static layout, the click track, and the low-res proxy. Split out of `preview.rs` (frame compositing) so each file stays focused. All reuse the warm renderer cache via `with_warm`.

## CamSample

```rust
#[derive(serde::Serialize)]
pub struct CamSample { pub t: u32, pub scale: f32, pub cx: f32, pub cy: f32, pub curx: f32, pub cury: f32 }
```

One sample of the camera curve at output time `t` (ms). `scale` is the zoom factor; `cx`/`cy` are the zoom centre and `curx`/`cury` the cursor position, both as 0..1 fractions of the *screen content* (i.e. of the `<video>` the editor plays), not of the output. The editor interpolates these (`camera.ts`) and the canvas compositor maps them through the zoom crop, so the zoom and cursor land correctly regardless of the screen panel's position in the layout.

## camera_track

```rust
#[tauri::command]
pub async fn camera_track(folder: String, app: tauri::AppHandle) -> Result<Vec<CamSample>, String>
```

Returns the exact camera curve over the whole timeline, one `CamSample` per output frame.

**Since the time remap** the samples are keyed by OUTPUT time: the command walks the renderer's `TimeMap::frame_plan` through `walk_plan` (warm-up, cut snaps and all, exactly as the exporter does), one `CamSample` per output frame with `t = j * 1000 / OUT_FPS`. The TS side does not receive the plan; it rebuilds the same map from the doc (`src/shared/math/remap.ts`) and looks the track up at `outOf(clip time)`.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`, the pattern `preview_frame` documents (`mod.md`). An earlier sweep left this sync on the grounds that its own body is pure math on a warm cache - true, but every `with_warm` command can land on the COLD path, where `FrameRenderer::new` decodes the event log, spawns up to three `ffprobe`/`ffmpeg` subprocesses, builds two wgpu pipelines and preps the cursor pack. The editor fires six of these commands on the same mount tick (`camera_track`, `preview_layout`, `preview_layouts`, `click_track`, `cursor_kinds`, `preview_bg`), so as sync commands the first paid that build on the UI thread and the other five queued behind it - the multi-hundred-ms-to-seconds freeze on opening a project, repeated on every aspect change. Because `tauri::State<'_, PreviewSession>` is not `'static` it cannot cross into `spawn_blocking`; the command takes `app: tauri::AppHandle` instead and re-derives the same managed state inside the closure via `app.state::<PreviewSession>()`. The JS call is unchanged - `AppHandle` is injected by Tauri, never passed from the frontend.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; all assets and `edit.json` derive from it.
- `app: tauri::AppHandle` - resolves the warm `FrameRenderer` cache (`PreviewSession`) from inside the `spawn_blocking` closure. *Why:* the curve reuses the same renderer as `preview_frame` (keyed by `edit.json` mtime), so it rebuilds only when an edit changes the timeline; `State<'_, _>` cannot cross a `spawn_blocking` boundary (see above).

### Returns

`Result<Vec<CamSample>, String>` - the per-frame samples over the WHOLE clip (not just the trim range - see below). Errors (as a string) if the renderer cannot be built (missing files).

### Implementation

1. Inside `with_warm` (the shared cache helper), read the duration as `meta.video_end - meta.video_start` - the recording's TRUE full length, NOT `trim.out_ms` (which, once a user actually trims, is a strict sub-range). This means scrubbing into a trimmed-out region still shows an animated camera curve instead of freezing on the last in-range sample; trim only clamps PLAYBACK (the frontend's `resolveTrim`-based pause/snap in `Editor.tsx`), not the curve data itself.
2. `reset_camera`, then `step_camera(video_start + t, OUT_STEP_MS)` walking the EXPORT's own frame index - `t = k * 1000 / OUT_FPS` for `k = 0, 1, 2, ...` up to that full duration. This is pure math (no decode), so it is instant. *Why the frame index and not a flat 16ms step:* `1000 / OUT_FPS` in integer math is 16, but the true 60fps period is 16.667, so a 16ms grid took 4.17% more steps per second than the export ever does. Both runs are stateful, so the preview drifted from the export by up to 28.7 source px (74.5 screen px) at sharp transitions - the preview quietly stopped being a preview. On the export's own grid, with the exact period handed to the filters, the two now agree to 0.000 px (`jank_probe_tests::preview_grid_vs_true_60fps`). Sample times stay whole milliseconds, which is what the editor's `camAt` interpolates between.
3. For each pose, map the zoom centre (`pose.cam.cx/cy`) and cursor (`pose.cur.x/y`) - both output coords - into `pose.scene.screen.rect` to get 0..1 screen-relative fractions, and push a `CamSample`.

## PreviewLayout

```rust
#[derive(serde::Serialize)]
pub struct PreviewLayout { pub screen: [f32; 4], pub radius: f32, pub cam: Option<[f32; 9]>, pub canvas: [u32; 2] }
```

The static export framing as fractions of the output: `screen` is the screen panel rect `[x, y, w, h]`, `radius` its corner radius (fraction of output *width*), and `cam` the webcam PiP rect+ring `[x, y, w, h, radius, ring_px, ring_r, ring_g, ring_b]` or `None` when the webcam is hidden. `ring_px` is a fraction of output width (0 = no ring); `ring_r/g/b` are 0..255 - mirrors `Panel.ring_px`/`ring_color` riding alongside its rect/radius (Task 9). `canvas` is the resolved preview frame's pixel dimensions (`Layout::resolve`'s output, following `EditDoc.aspect`) - the editor sizes its `<canvas>` + `.e-stage` aspect-ratio from this instead of a hardcoded 16:9/1280x720. The editor's canvas compositor uses these so the preview frames the screen, webcam, and ring exactly like the export instead of guessing.

## preview_layout

```rust
#[tauri::command]
pub async fn preview_layout(folder: String, app: tauri::AppHandle) -> Result<PreviewLayout, String>
```

Returns the `PreviewLayout` for the recording. `async` + `spawn_blocking` for the same reason as `camera_track` above.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) inside the blocking closure. *Why:* the layout comes from the same scene the export uses; reusing the cache avoids a rebuild.

### Returns

`Result<PreviewLayout, String>` - the screen/webcam framing fractions (+ ring) + the resolved canvas size. Errors (as a string) if the renderer cannot be built.

### Implementation

1. Inside `with_warm`, `reset_camera`, then `step_camera(video_start, OUT_STEP_MS)` to get the scene at t=0 (the unzoomed base layout).
2. Divide `pose.scene.screen.rect` and `pose.scene.camera.rect` (+ radii, + `ring_px`) by `c.meta.out_w`/`out_h` to get fractions; set `cam` to `None` when `pose.scene.camera.alpha <= 0.5`; set `canvas: [c.meta.out_w, c.meta.out_h]` (the warm renderer's own resolved size).

## ClickSample

```rust
#[derive(serde::Serialize)]
pub struct ClickSample { pub t: u32, pub x: f32, pub y: f32 }
```

One click ripple: output time `t` (ms) and `x`/`y` as 0..1 fractions of the screen content - the same basis as `CamSample`'s cursor, so the editor draws the ripple exactly where the cursor clicked (mapped through the current zoom crop).

## click_track

```rust
#[tauri::command]
pub async fn click_track(folder: String, app: tauri::AppHandle) -> Result<Vec<ClickSample>, String>
```

Returns the click (mouse-down) track over the whole timeline, for click-ripple effects in the editor preview that match the export's click FX. `async` + `spawn_blocking` for the same reason as `camera_track` above.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) inside the blocking closure. *Why:* clicks come from the renderer's owned event log; reusing the cache avoids reloading it.

### Returns

`Result<Vec<ClickSample>, String>` - the click samples (output ms + screen-content position). Errors (as a string) if the renderer cannot be built.

### Implementation

1. Inside `with_warm`, call `renderer.click_track(video_start)`.
2. That filters the owned event log to mouse-down events, maps each into a screen-content fraction (`Cursor::clicks`, via `to_frame` then divide by the screen size), and shifts the event time to output time (`et + events_ms - video_start`, dropping any that land before 0).

## ensure_proxy

```rust
#[tauri::command]
pub async fn ensure_proxy(folder: String, height: u32) -> Result<String, String>
```

Tauri IPC command (Task 41: off the main thread). `spawn_blocking(ensure_proxy_blocking)`, `.await`ed, join failure mapped to `Err(String)` - see `export::preview::thumbs`'s module doc for why (same freeze mechanism as Task 40's `ai::commands`, a proxy transcode being the essential preprocessing step run right on a project OPEN). `export::preview::preprocess::run` calls `ensure_proxy_blocking` directly since it already runs off-thread on its own `std::thread::spawn`.

## ensure_proxy_blocking

```rust
pub(crate) fn ensure_proxy_blocking(folder: String, height: u32) -> Result<String, String>
```

Ensures a low-res preview proxy `preview_<height>_rt.mp4` exists (transcoded once, cached) and returns its path, so the editor plays a light proxy instead of the raw (often 4K) capture. The `_rt` ("re-timed") proxy is stretched to the real recording duration (see below), because the capture encoder tags frames at a fixed nominal fps that's usually faster than the real capture rate, so `video.mp4` plays sped up.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates the source `video.mp4` and where the proxy is written.
- `height: u32` - target proxy height (e.g. 480/720/1080). *Why:* the editor picks a resolution to suit the display; clamped to 240..2160 and rounded even.

### Returns

`Result<String, String>` - the proxy file path. Errors (as a string) if the ffmpeg transcode fails.

### Implementation

1. Build the proxy path `preview_<h>_rt.mp4`. If it already exists, return it.
2. Compute the stretch factor `k = real / enc`, where `real = edit::seed::true_duration_ms(paths) / 1000` (the recording's TRUE full duration, NOT `trim.out_ms` - which once a user actually trims is a sub-range, not the whole clip the proxy must cover) and `enc = probe_duration(video.mp4)` (the file's sped-up encoded length). When `|k - 1| > 0.02`, the video filter is `scale=-2:<h>,setpts=<k>*PTS` (stretch to true speed); otherwise just `scale=-2:<h>`.
3. Run ffmpeg with that filter (`libx264 -preset ultrafast -g 60 -crf 27`, `yuv420p`, `+faststart`, no audio; see `ensure_proxy_with_progress` for why those two) via `ffcmd_bg` (below-normal priority, so the transcode yields to the UI instead of freezing it) inside `process::proc::generate_once` (the pass is skipped if the proxy already exists and is serialized against the other editor-media transcodes, so `preprocess_project`'s post-record pass and the editor's own lazy `ensure_proxy` don't transcode the 4K source twice or storm the CPU as the editor opens), writing to a `process::proc::tmp_sibling` then atomically renaming onto the proxy path, so an editor opening WHILE preprocessing is still running never loads a half-transcoded file. Return the path on success. *Why re-time the proxy rather than the master:* the export already corrects timing via `sync.json` frame selection (independent of `video.mp4`'s embedded PTS), so only the natively-played preview needs the fix; the `_rt` filename also invalidates any older sped-up proxy.

## ensure_proxy_with_progress

```rust
pub(crate) fn ensure_proxy_with_progress(folder: String, height: u32, on_progress: &dyn Fn(u32)) -> Result<String, String>
```

`ensure_proxy_blocking` (which is this with a no-op callback) reporting the transcode's own progress: ffmpeg runs with `-progress pipe:1 -nostats`, its stdout is read line by line on the calling thread, and every `out_time_us=` line becomes `on_progress(proxy_pct(line, real))`. *Why:* the proxy IS the wait between Stop and the editor (`preprocess::essential`), and a pill that sat at 0% for a minute on a long take read as hung.

### Encoder settings (measured on a 5-minute 1080p60 take, 60s slices, 2026-09-14)

- **No `-hwaccel`.** Hardware decode into the CPU `scale`/`setpts` filters measured 3x SLOWER than CPU decode (the surface download dominates), and `auto` once picked a frame format the filters rejected outright. CPU decode plus scale is the floor here, ~1.3s per 60s of video.
- **`-preset ultrafast`**, not `veryfast`: 1.8s per 60s against 2.5s, and a preview does not need the compression. Hardware encoders did not help - the encode was never the wall.
- **`-g 60`**: a keyframe every second, so the editor's seeks land fast and the filmstrip can decode keyframes only (`thumbs::ensure_thumbs_blocking`).

## proxy_pct

```rust
pub(crate) fn proxy_pct(line: &str, real_secs: f64) -> Option<u32>
```

One line of ffmpeg's `-progress` stream -> percent of the proxy written, for `out_time_us=` lines only (`None` for the rest of its key=value chatter: `frame=`, `fps=`, `progress=`...). Output time is already stretched to the real duration by `setpts`, so it is measured against `real_secs` (floored at 0.05). Capped at 99: the last point is the caller's, once the file has been renamed into place. Tests: `preprocess.rs`.

