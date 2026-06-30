# src-tauri/src/export/camera.rs

Stateful virtual camera that smoothly damps its zoom scale and pan center toward per-frame setpoints derived from the active zoom region and cursor position. Produces a `Camera` value each frame consumed by the compositors and FX overlay. The most-recently-started active region always wins, so a new click preempts an older region's zoom-out immediately.

## CameraSim

```rust
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32 }
```

The virtual camera's mutable state.

- `frame_w: u32, frame_h: u32` - output frame dimensions. *Why:* used to compute the dead-band radius in the hold phase and to clamp the center so the viewport stays in-frame.
- `cx: f32, cy: f32` - current smoothed camera center in output pixels. *Why:* carried between frames; the damping step moves `cx/cy` toward the setpoint without teleporting.
- `scale: f32` - current smoothed zoom multiplier. *Why:* same - exponential approach keeps motion continuous across region transitions.

### Used by

- `src-tauri/src/export/exporter.rs` - one `CameraSim` per export; `step` is called in the frame loop.

## CameraSim::new

```rust
pub fn new(frame_w: u32, frame_h: u32) -> Self
```

Creates a `CameraSim` at scale 1.0, centered at `(frame_w/2, frame_h/2)`.

### Inputs

- `frame_w: u32, frame_h: u32` - output frame size. *Why:* initializes the center to the frame midpoint so the first frame is unzoomed and centered.

### Returns

A new `CameraSim` with `scale=1.0`.

## CameraSim::step

```rust
pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera
```

Advances the simulation one frame at time `t_ms` and returns the resulting `Camera`.

### Inputs

- `t_ms: u32` - milliseconds from the event log origin. *Why:* used to determine which phase (zoom-in, hold, zoom-out) the active region is in.
- `cursor: FramePoint` - current cursor position in panel output pixels (from `coordmap::to_panel`). *Why:* during the hold phase the camera pans only when the cursor exits the dead band.
- `regions: &[ZoomRegion]` - all zoom regions for this export. *Why:* iterated in reverse (`rev().find(...)`) so the most-recently-started region that overlaps `t_ms` wins.
- `cfg: &ZoomConfig` - provides `follow_damping` and `target_scale`. *Why:* `follow_damping` is the per-frame exponential step size; `target_scale` is used in the dead-band radius calculation.

### Returns

`Camera` with the post-step `cx`, `cy`, and `scale`. The returned value is owned by the caller (exporter) each frame.

### Implementation

1. Find the active region: `regions.iter().rev().find(|r| t_ms >= r.start_ms && t_ms <= r.end_ms)`. Most-recently-started wins.
2. Compute the setpoint `(target_scale, target_cx, target_cy)`:
   - No active region: `(1.0, fw/2, fh/2)` - return to full-frame center.
   - Zoom-in phase (`t_ms < r.start_ms + r.zoom_in_ms`): `(r.target_scale, r.anchor.x, r.anchor.y)` - home toward the click anchor.
   - Zoom-out phase (`t_ms >= r.end_ms - r.zoom_out_ms`): `(1.0, self.cx, self.cy)` - release in place; the clamping re-centers as scale approaches 1.
   - Hold phase (between zoom-in end and zoom-out start): keep `r.target_scale`; compute the dead band `mx = fw / (2*target_scale) * 0.4`, `my = fh / (2*target_scale) * 0.4`. Pan the center only when `|cursor - center| > dead_band`; inside the band, hold `self.cx/cy` as the target.
3. Apply exponential damping with the same coefficient `k = cfg.follow_damping` to both scale and center so zoom and pan move in lockstep.
4. Clamp `self.cx` and `self.cy` so the viewport (half-size = `frame / (2 * scale)`) stays within the frame bounds.
5. Return `Camera { cx: self.cx, cy: self.cy, scale: self.scale }`.

### Behaviors worth knowing

- `no_region_is_full_frame` (unit test): with no regions, `step` at t=0 returns scale=1.0, center at frame midpoint.
- `scale_damps_in_to_target_then_back_out` (unit test): scale starts below 1.5 (damping, not instant), reaches 2.0 during hold (within 0.05), and returns to ~1.0 after zoom-out.
- `newer_region_preempts_older_overlap` (unit test): with two overlapping regions, the later-starting one's anchor wins when t is in both.
- `center_clamps_inside_frame` (unit test): an anchor at (0,0) with scale~2 keeps `cx >= 200`, `cy >= 150` (half-viewport of an 800x600 frame).
