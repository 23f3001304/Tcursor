# src-tauri/src/export/preview_track.rs

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
pub fn camera_track(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<CamSample>, String>
```

Returns the exact camera curve over the whole timeline, one `CamSample` per output frame.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; all assets and `edit.json` derive from it.
- `session: State<PreviewSession>` - the warm `FrameRenderer` cache. *Why:* the curve reuses the same renderer as `preview_frame` (keyed by `edit.json` mtime), so it rebuilds only when an edit changes the timeline.

### Returns

`Result<Vec<CamSample>, String>` - the per-frame samples from t=0 to the trim end. Errors (as a string) if the renderer cannot be built (missing files).

### Implementation

1. Inside `with_warm` (the shared cache helper), read the trim duration from `load_or_seed(...).trim.out_ms`.
2. `reset_camera`, then `step_camera(video_start + t)` for `t` stepping by `1000/OUT_FPS` up to the trim duration. This is pure math (no decode), so it is instant.
3. For each pose, map the zoom centre (`pose.cam.cx/cy`) and cursor (`pose.cur.x/y`) - both output coords - into `pose.scene.screen.rect` to get 0..1 screen-relative fractions, and push a `CamSample`.

## PreviewLayout

```rust
#[derive(serde::Serialize)]
pub struct PreviewLayout { pub screen: [f32; 4], pub radius: f32, pub cam: Option<[f32; 5]> }
```

The static export framing as fractions of the output: `screen` is the screen panel rect `[x, y, w, h]`, `radius` its corner radius (fraction of output *width*), and `cam` the webcam PiP rect `[x, y, w, h, radius]` or `None` when the webcam is hidden. The editor's canvas compositor uses these so the preview frames the screen and webcam exactly like the export instead of guessing.

## preview_layout

```rust
#[tauri::command]
pub fn preview_layout(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<PreviewLayout, String>
```

Returns the `PreviewLayout` for the recording.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `session: State<PreviewSession>` - the warm renderer cache. *Why:* the layout comes from the same scene the export uses; reusing the cache avoids a rebuild.

### Returns

`Result<PreviewLayout, String>` - the screen/webcam framing fractions. Errors (as a string) if the renderer cannot be built.

### Implementation

1. Inside `with_warm`, `reset_camera`, then `step_camera(video_start)` to get the scene at t=0 (the unzoomed base layout).
2. Divide `pose.scene.screen.rect` and `pose.scene.camera.rect` (+ radii) by the output dimensions to get fractions; set `cam` to `None` when `pose.scene.camera.alpha <= 0.5`.

## HoldSpan

```rust
#[derive(serde::Serialize)]
pub struct HoldSpan { pub start_ms: u32, pub end_ms: u32 }
```

One recorded effect-hold interval in OUTPUT time (ms) - same time basis as `ClickSample` and `CamSample`. The editor preview applies the same 250ms fade ramp over `[start_ms, end_ms)` that the export's `hold_alpha` uses, so a recorded hold lights up identically.

## spotlight_holds

```rust
#[tauri::command]
pub fn spotlight_holds(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<HoldSpan>, String>
```

Returns the recorded spotlight-hold intervals (the hotkey spotlight held during capture) in output time, so the editor preview lights held spotlights exactly like the export - not just editor-added effect regions. Mirrors the hold half of the export's `s_alpha = max(settings, hold_alpha(actions), region_alpha)`.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording (its `actions.json` carries the holds).
- `session: State<PreviewSession>` - the warm renderer cache. *Why:* the holds come from the renderer's owned action log; reusing the cache avoids reloading it.

### Returns

`Result<Vec<HoldSpan>, String>` - the held intervals clipped to `[0, dur]`, output-time. Errors (as a string) if the renderer cannot be built.

### Implementation

1. Inside `with_warm`, read the trim duration from `load_or_seed(...).trim.out_ms`.
2. Compute the event→output offset `off = events_ms - video_start` (the inverse of `step_camera`'s `ev_t = t - events_ms`, identical to `click_track`).
3. Call `hold::hold_spans` on `renderer.actions()` with the `SpotlightHoldStart`/`SpotlightHoldEnd` predicates (an unpaired open hold runs to the output end), then shift each span by `off` and clip to `[0, dur]`, dropping spans that fall entirely outside.

## ClickSample

```rust
#[derive(serde::Serialize)]
pub struct ClickSample { pub t: u32, pub x: f32, pub y: f32 }
```

One click ripple: output time `t` (ms) and `x`/`y` as 0..1 fractions of the screen content - the same basis as `CamSample`'s cursor, so the editor draws the ripple exactly where the cursor clicked (mapped through the current zoom crop).

## click_track

```rust
#[tauri::command]
pub fn click_track(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<ClickSample>, String>
```

Returns the click (mouse-down) track over the whole timeline, for click-ripple effects in the editor preview that match the export's click FX.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `session: State<PreviewSession>` - the warm renderer cache. *Why:* clicks come from the renderer's owned event log; reusing the cache avoids reloading it.

### Returns

`Result<Vec<ClickSample>, String>` - the click samples (output ms + screen-content position). Errors (as a string) if the renderer cannot be built.

### Implementation

1. Inside `with_warm`, call `renderer.click_track(video_start)`.
2. That filters the owned event log to mouse-down events, maps each into a screen-content fraction (`Cursor::clicks`, via `to_frame` then divide by the screen size), and shifts the event time to output time (`et + events_ms - video_start`, dropping any that land before 0).

## ensure_proxy

```rust
#[tauri::command]
pub fn ensure_proxy(folder: String, height: u32) -> Result<String, String>
```

Ensures a low-res preview proxy `preview_<height>_rt.mp4` exists (transcoded once, cached) and returns its path, so the editor plays a light proxy instead of the raw (often 4K) capture. The `_rt` ("re-timed") proxy is stretched to the real recording duration (see below), because the capture encoder tags frames at a fixed nominal fps that's usually faster than the real capture rate, so `video.mp4` plays sped up.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates the source `video.mp4` and where the proxy is written.
- `height: u32` - target proxy height (e.g. 480/720/1080). *Why:* the editor picks a resolution to suit the display; clamped to 240..2160 and rounded even.

### Returns

`Result<String, String>` - the proxy file path. Errors (as a string) if the ffmpeg transcode fails.

### Implementation

1. Build the proxy path `preview_<h>_rt.mp4`. If it already exists, return it.
2. Compute the stretch factor `k = real / enc`, where `real = trim.out_ms / 1000` (the seeded clip duration = the real capture span the export uses) and `enc = probe_duration(video.mp4)` (the file's sped-up encoded length). When `|k - 1| > 0.02`, the video filter is `scale=-2:<h>,setpts=<k>*PTS` (stretch to true speed); otherwise just `scale=-2:<h>`.
3. Run ffmpeg with that filter (`libx264 -preset veryfast -crf 27`, `yuv420p`, `+faststart`, no audio), writing to a `win::proc::tmp_sibling` then atomically renaming onto the proxy path, so the editor opening during the post-record `prewarm` never loads a half-transcoded file. Return the path on success. *Why re-time the proxy rather than the master:* the export already corrects timing via `sync.json` frame selection (independent of `video.mp4`'s embedded PTS), so only the natively-played preview needs the fix; the `_rt` filename also invalidates any older sped-up proxy.
