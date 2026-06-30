# src-tauri/src/export/render.rs

Reusable per-frame renderer that owns all per-export setup except the raw decoders and the encoder sink. Extracted from `exporter.rs` so the same compositing code can serve both the export loop and the preview engine (Task 2). The exporter calls `FrameRenderer::new` once, then `step_camera` + `composite_at` once per output frame. The preview engine fast-forwards `step_camera` from 0 to a target time (cheap math, no decode) and composites once.

## OUT_FPS

```rust
pub const OUT_FPS: u64 = 60
```

Constant output frame rate (60 fps). Defined here so both `exporter.rs` and future preview code share a single source of truth for the encode rate.

## RenderMeta

```rust
pub struct RenderMeta {
    pub tl: Timeline,
    pub video_start: u64,
    pub video_end: u64,
    pub out_w: u32,
    pub out_h: u32,
    pub sw: u32,
    pub sh: u32,
    pub screen_bytes: usize,
    pub webcam_size: u32,
    pub audio_offset_ms: i32,
}
```

All information the export (or preview) loop needs to set up its raw decoders and drive the frame loop, returned by `FrameRenderer::new` so the caller never needs to re-read the project files.

**Fields and why each is here:**

- `tl: Timeline` - per-frame wall-clock timestamps and audio offsets from `build_timeline`. *Why:* the export loop indexes this to advance the screen decoder to the captured frame active at each output time.
- `video_start: u64` - timestamp (ms) of the first captured frame. *Why:* the loop's time variable `t = video_start + k * 1000 / OUT_FPS`.
- `video_end: u64` - timestamp of the last captured frame (at least `video_start + 1`). *Why:* `total_out = (video_end - video_start) * OUT_FPS / 1000` drives the loop bound.
- `out_w: u32`, `out_h: u32` - output canvas dimensions in pixels. *Why:* `FrameRenderer` owns the layout after `new()`, so the caller gets these from meta rather than re-reading the layout struct.
- `sw: u32`, `sh: u32` - raw screen capture dimensions (probe result). *Why:* needed to allocate the `screen_bytes`-sized decode buffer.
- `screen_bytes: usize` - `sw * sh * 4`: exact BGRA buffer size for the screen decoder. *Why:* pre-computed to avoid re-doing the multiply at every `RawDecoder::spawn` call.
- `webcam_size: u32` - square side length (pixels) for webcam decode, capped to 1440. *Why:* `RawDecoder::spawn` for the webcam takes this as the `Some(size)` resize argument; returned here so the exporter can allocate the matching buffer.
- `audio_offset_ms: i32` - the user's manual mic-sync nudge from settings. *Why:* carried here so `exporter.rs` does not need to reload the edit doc a second time after `new()`.

### Used by

`exporter::export` reads every field to set up the decoders and drive `0..=total_out`.

## FramePose

```rust
pub struct FramePose {
    pub ev_t: u32,
    pub scene: Scene,
    pub cur: FramePoint,
    pub cam: Camera,
}
```

The resolved camera and scene for one output frame, returned by `step_camera` and passed unchanged to `composite_at`. Grouping these four values as a struct avoids passing them as separate arguments and lets the preview engine inspect the pose (e.g. to know zoom scale) without compositing.

**Fields and why each is here:**

- `ev_t: u32` - event-relative time in ms (`t - events_ms`). *Why:* all per-frame lookups (cursor, layout, FX, cursor sprite) key on this value; pre-computing it in `step_camera` avoids re-doing the subtraction in `composite_at`.
- `scene: Scene` - the two-panel layout (screen + camera rects/alphas) possibly modified by camera-shrink. *Why:* the compositor, FX renderer, and cursor draw all need the scene; carrying it in the pose avoids re-calling `track.scene_at`.
- `cur: FramePoint` - cursor position mapped into the screen panel (output pixels). *Why:* needed by the compositor for the zoom setpoint, by FX for click-ring placement, and by the cursor sprite draw.
- `cam: Camera` - zoom center + scale for this frame. *Why:* the compositor crops and resizes to this, and the FX + cursor layers project through it.

### Used by

`composite_at` reads all four fields. The preview engine (Task 2) will inspect `cam` and `scene` to derive the zoom level for display.

## FrameRenderer

```rust
pub struct FrameRenderer { /* private fields */ }
```

Owns all per-export setup that is not a raw decoder or encoder sink: event log, settings, layout, zoom regions, background pixels, compositor, FX renderer, camera sim, cursor state, cursor sprite set, and action log. Initialized once per export/preview; then drives per-frame rendering via `step_camera` and `composite_at`.

Private fields include:
- `settings: Settings` - cloned from the edit doc; drives all effect toggles per frame.
- `cfg: ZoomConfig` - derived from `settings.zoom`; used by `CameraSim::step`.
- `layout: Layout` - output canvas dimensions and screen-panel geometry.
- `track: LayoutTrack` - resolves the active `Scene` at any event time.
- `regions: Vec<ZoomRegion>` - re-anchored zoom regions for `CameraSim::step`.
- `bg: Vec<u8>` - decoded background BGRA pixels; passed to compositor each frame.
- `compositor: Box<dyn Compositor>` - GPU or CPU compositor, selected once at init.
- `fx: Box<dyn FxRenderer>` - GPU or CPU FX renderer.
- `sim: CameraSim` - stateful; must be stepped in frame-loop order.
- `events: Vec<MouseEvent>` - owned copy of the mouse log (used by both cursor logic and `fx_state::render`).
- `screen: ScreenInfo` - capture monitor geometry for coordinate mapping.
- `cur_idx / cur_sx / cur_sy / cur_primed` - cursor smooth-filter state, equivalent to `Cursor<'a>` internal fields. Stored inline because `FrameRenderer` owns the event log and a self-referential borrow is not possible without unsafe.
- `cprep: Option<CursorPrep>` - decoded cursor sprite set; `None` for non-Enhanced styles.
- `actions: Vec<ActionEvent>` - action log; used by `fx_state::render` for spotlight/hold/caption.
- `sw, sh: u32` - screen capture dimensions.
- `events_ms: u64` - wall-clock offset of event-time 0; used to convert `t` to `ev_t`.

### Used by

`exporter::export` (the export loop), and Task 2 `preview.rs` (the preview engine).

## FrameRenderer::new

```rust
pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32) -> Result<(Self, RenderMeta)>
```

Loads the edit doc and event log, resolves all per-export setup, and returns both the renderer and the metadata the caller needs to spawn decoders.

### Inputs (what, and why it is needed)

- `paths: &ProjectPaths` - root of the project folder. *Why:* all asset paths (`events.json`, `actions.json`, `video`, `webcam`, `cursor.json`, `sync.json`, `edit.json`) are derived from it.
- `layout: Layout` - output canvas geometry. *Why:* taken by value so `new` can pass it to `LayoutTrack::new`, `CameraSim::new`, and the compositor constructor without copying; the exporter uses `Layout::default()` and the preview engine will pass a smaller layout.
- `fps: u32` - capture frame rate. *Why:* passed to `build_timeline` as a last-resort denominator when `sync.json` is absent and no audio duration is available; matches the `fps` parameter of `exporter::export`.

### Returns

`Result<(FrameRenderer, RenderMeta)>`. Errors on `events.json` load failure or `probe_dims` failure (video file missing or unreadable).

### Implementation

Follows the same sequence as the original `exporter::export` setup block (lines ~32-107), with the encoder/sink and the `RawDecoder::spawn` calls excluded. The webcam-size calculation (`max_cam` over all layout modes, capped to 1440) is done here and returned in `RenderMeta.webcam_size` so the exporter does not need to repeat it.

## FrameRenderer::step_camera

```rust
pub fn step_camera(&mut self, t: u64) -> FramePose
```

Advances the camera simulation to output time `t` and returns the resolved pose. Cheap: only arithmetic, no I/O. Must be called in ascending `t` order because `CameraSim` and the cursor index are stateful.

### Inputs (what, and why it is needed)

- `t: u64` - output time in ms (wall-clock, same epoch as `RenderMeta.video_start`). *Why:* the camera sim and cursor interpolation both key on absolute time, then subtract `events_ms` to get the event-relative `ev_t`.

### Returns

`FramePose` with `ev_t`, `scene` (possibly camera-shrunk), `cur` (panel-mapped cursor), and `cam` (damped camera).

### Implementation

Replicates `Cursor::at` inline (index advance + exponential low-pass at `A=0.35`) using owned event data, because `FrameRenderer` owns the event log and cannot hold a `Cursor<'a>` that borrows from itself. Then calls `LayoutTrack::scene_at`, `to_panel`, `CameraSim::step`, applies the CameraOnly identity override (`scene.screen.alpha < 0.5`), and applies `shrink_camera` when enabled and the screen panel is dominant.

## FrameRenderer::composite_at

```rust
pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8],
                    webcam: Option<(&[u8], u32, u32)>) -> Vec<u8>
```

Runs the full compositor + FX + cursor pipeline for one frame. Expensive (GPU or multi-core CPU work). Returns a BGRA buffer of `out_w * out_h * 4` bytes.

### Inputs (what, and why it is needed)

- `pose: &FramePose` - the resolved pose from `step_camera`. *Why:* the compositor, FX renderer, and cursor draw all need the same scene/camera/cursor values; passing one struct keeps the call site clean.
- `screen: &[u8]` - decoded screen frame as BGRA. *Why:* the compositor takes the raw screen pixels and places them into the output canvas.
- `webcam: Option<(&[u8], u32, u32)>` - decoded webcam frame (bytes, width, height) or `None`. *Why:* the compositor uses this for the camera panel; `None` when webcam is absent or exhausted.

### Returns

`Vec<u8>` of BGRA pixels (`out_w * out_h * 4` bytes). Allocated by the compositor on each call.

### Implementation

Three sequential stages (identical to the original exporter loop body, lines ~138-144):
1. `compositor.composite(...)` - places screen + webcam onto the background with zoom crop and panel rounding.
2. `fx_state::render(...)` - applies click rings, spotlight, video FX, and captions.
3. `cursorset::draw(...)` (if `cprep.is_some()`) - blits the Enhanced cursor sprite with motion trail, bounce, and panel clipping.

## FrameRenderer::bg

```rust
pub fn bg(&self) -> &[u8]
```

The export background buffer (BGRA, `out_w * out_h * 4` bytes) - the same mesh/gradient the compositor draws under the screen. Exposed so the editor preview (`preview_bg`) can paint the exact same background the export uses.

## FrameRenderer::click_track

```rust
pub fn click_track(&self, video_start: u64) -> Vec<(u32, f32, f32)>
```

Click (mouse-down) events mapped to `(output_ms, x, y)` where `x`/`y` are 0..1 fractions of the screen content - so the editor preview can draw click ripples that match the export's click FX.

### Inputs

- `video_start: u64` - the first video frame's capture timestamp (from `RenderMeta`). *Why:* the output time inverts `step_camera`'s `ev_t = t - events_ms` offset, so a click at event time `et` shows at output `et + events_ms - video_start`.

### Returns

`Vec<(u32, f32, f32)>` - one tuple per mouse-down event whose output time is >= 0, in ascending time order. Positions come from `Cursor::clicks` (same basis as the smoothed cursor), so ripples land exactly where the cursor clicked.

## select_compositor

```rust
pub fn select_compositor(layout: &Layout) -> Box<dyn Compositor>
```

Returns a `GpuCompositor` when a GPU adapter is available and construction succeeds; otherwise a `CpuCompositor`. Never fails. Public so the preview engine can use it directly if needed.
