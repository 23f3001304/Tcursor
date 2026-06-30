# src-tauri/src/export/easing.rs

Single public function mapping a normalized time value through a curve selected by an `Easing` variant. Pure and stateless - used by the camera simulator and any zoom-region interpolation that needs a curve.

## ease

```rust
pub fn ease(e: Easing, t: f32) -> f32
```

Maps `t` in [0, 1] to an eased value in [0, 1]. Input is clamped to [0, 1] before the curve is applied, so out-of-range values are safe.

### Inputs

- `e: Easing` - which curve to apply. *Why:* each zoom region carries its own `easing` field so individual regions can use different curves without changing the call site.
- `t: f32` - normalized time in 0..1. *Why:* the caller is responsible for computing `t` from the region timestamps; `ease` does only the curve mapping.

### Returns

`f32` in [0, 1]. Both endpoints are exact: `ease(_, 0.0) == 0.0` and `ease(_, 1.0) == 1.0`.

### Implementation

1. Clamp `t` to [0, 1].
2. Match on `e`:
   - `Easing::Linear`: return `t` unchanged.
   - `Easing::Smooth`: return `1.0 - (1.0 - t).powi(3)` (ease-out cubic). *Why ease-out:* the zoom decelerates into its target position so it feels like it "lands", rather than approaching linearly which feels mechanical.
   - `Easing::Spring { .. }`: currently returns `1.0 - (1.0 - t).powi(3)`, identical to `Smooth`. The spring parameters (`stiffness`, `damping`) are accepted but ignored. *Why:* a real spring simulation is deferred to M3+; the variant exists in the type system now so recorded data with spring easing is forward-compatible.

### Behaviors worth knowing

- `smooth_hits_endpoints_and_eases_out` (unit test): `ease(Smooth, 0.0)=0.0`, `ease(Smooth, 1.0)=1.0`, and `ease(Smooth, 0.5) > 0.5` (ease-out is past the linear midpoint at t=0.5, reaching 0.875).
- `linear_is_identity_clamped` (unit test): `ease(Linear, 0.3)=0.3`; out-of-range `-1.0` clamps to `0.0`, `2.0` clamps to `1.0`.

### Used by

- `src-tauri/src/export/camera.rs` - `CameraSim::step` may call `ease` for region-specific curves.
- `src-tauri/src/export/layout.rs` - `LayoutTrack` uses it for cross-dissolve transition easing.
