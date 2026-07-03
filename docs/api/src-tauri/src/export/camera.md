# src-tauri/src/export/camera.rs

Stateful virtual camera that follows a deterministic eased scale curve fully contained within each zoom region: scale ramps 1 -> target over `zoom_in_ms`, holds, then ramps target -> 1 over `zoom_out_ms`, reaching exactly 1.0 at `end_ms`. Produces a `Camera` value each frame consumed by the compositors and FX overlay. The most-recently-started active region always wins, so a new click preempts an older region's zoom-out immediately.

## ease

```rust
fn ease(e: Easing, p: f32) -> f32
```

Normalized easing curve mapping progress `p` in `[0,1]` to `[0,1]`.

### Inputs

- `e: Easing` - which curve to apply. *Why:* `Linear` is identity; `Smooth` is smoothstep (`3p^2 - 2p^3`); `Spring { .. }` is ease-out-back, overshooting slightly past 1 near the end before settling (the `stiffness`/`damping` fields are unused - only the variant tag matters).
- `p: f32` - progress, clamped to `[0,1]` before easing. *Why:* callers pass raw fractions that can fall slightly outside range at the boundaries; clamping avoids NaN/overshoot from the cubic terms.

### Returns

The eased progress value, `~0` at `p=0` and `~1` at `p=1` (with `Spring` briefly exceeding 1).

## fit_durations

```rust
fn fit_durations(zi: u32, zo: u32, span: u32) -> (u32, u32)
```

Shrinks `(zi, zo)` proportionally so their sum never exceeds `span`, keeping both ramps inside the region.

### Inputs

- `zi: u32, zo: u32` - requested zoom-in/zoom-out durations in ms. *Why:* a region's configured durations can exceed its own `end_ms - start_ms` span (e.g. a very short region), which would otherwise make the ramps overlap or run past the region.
- `span: u32` - the region's total duration (`end_ms - start_ms`, at least 1). *Why:* the upper bound the two ramps must fit under.

### Returns

`(zi, zo)` unchanged if `zi + zo <= span` (or if both are 0); otherwise both scaled down by the same factor `span / (zi + zo)` so they sum to `span`.

## CameraSim

```rust
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32 }
```

The virtual camera's mutable state.

- `frame_w: u32, frame_h: u32` - output frame dimensions. *Why:* used to compute the dead-band radius in the hold phase and to clamp the center so the viewport stays in-frame.
- `cx: f32, cy: f32` - current camera center in output pixels. *Why:* carried between frames; eased directly during zoom-in, panned during hold, and held (then reclamped) during zoom-out.
- `scale: f32` - current zoom multiplier. *Why:* carried between frames but recomputed deterministically from `t_ms` each call, not damped from the prior value.

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
2. No active region: snap `scale = 1.0`, `cx/cy` to the frame midpoint.
3. Active region `r`: compute `span = (r.end_ms - r.start_ms).max(1)` and fit the ramps with `fit_durations(r.zoom_in_ms, r.zoom_out_ms, span)` so `zi + zo <= span`.
   - Zoom-in phase (`t_ms < r.start_ms + zi`): `e = ease(r.easing, (t_ms - r.start_ms) / zi)`; `scale = 1 + (target_scale - 1) * e`; `cx/cy` eased in lockstep from the frame midpoint toward `r.anchor`.
   - Zoom-out phase (`t_ms >= r.end_ms - zo`): `e = ease(r.easing, (r.end_ms - t_ms) / zo)`; `scale = 1 + (target_scale - 1) * e`, which is exactly `1.0` when `t_ms == r.end_ms`. Center is left as-is (the in-frame clamp below re-centers it as scale shrinks toward 1).
   - Hold phase (between zoom-in end and zoom-out start): `scale = r.target_scale` (fixed, no easing); compute the dead band `mx = fw / (2*target_scale) * 0.4`, `my = fh / (2*target_scale) * 0.4`, pan the center toward the cursor only when it exits the band, damped by `cfg.follow_damping`.
4. Clamp `self.cx` and `self.cy` so the viewport (half-size = `frame / (2 * scale)`) stays within the frame bounds.
5. Return `Camera { cx: self.cx, cy: self.cy, scale: self.scale }`.

### Behaviors worth knowing

- `no_region_is_full_frame` (unit test): with no regions, `step` at t=0 returns scale=1.0, center at frame midpoint.
- `scale_is_one_at_region_end_and_after` (unit test): scale is exactly 1.0 (within 1e-3) at `t_ms == end_ms` and stays 1.0 for any `t_ms` after the region ends - the zoom-out never spills past the region's bound.
- `scale_reaches_target_during_hold` (unit test): scale equals `target_scale` exactly (within 1e-3) at a hold-phase timestamp, since hold no longer damps toward the target.
- `ramp_in_is_monotonic_and_bounded` (unit test): scale climbs monotonically from 1.0 toward (but not reaching) `target_scale` during the zoom-in ramp.
- `durations_that_exceed_span_are_scaled_to_fit` (unit test): a region whose `zoom_in_ms + zoom_out_ms` exceeds its own span still reaches exactly 1.0 at `end_ms` (via `fit_durations` shrinking both ramps proportionally) and never produces NaN.
- `newer_region_preempts_older_overlap` (unit test): with two overlapping regions, the later-starting one's anchor wins when t is in both.
- `center_clamps_inside_frame` (unit test): an anchor at (0,0) with scale~2 keeps `cx >= 200`, `cy >= 150` (half-viewport of an 800x600 frame).
