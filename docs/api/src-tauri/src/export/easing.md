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
   - `Easing::EaseIn`: `t*t` (quadratic accelerate - slow start).
   - `Easing::EaseOut`: `t*(2-t)` (quadratic decelerate - slow finish).
   - `Easing::EaseInOut`: `2t^2` for `t<0.5`, else `1-2(1-t)^2` (symmetric). *Why:* user-selectable camera-move transitions need the full accelerate/decelerate/both set; `camera::ease` mirrors these exactly (and so does the TS `ease` for the preview).
   - `Easing::Cubic { x1, y1, x2, y2 }`: delegates to `export::cubic::eval` - the same evaluation `camera::ease` uses, since a custom bezier has only one definition.
   - `Easing::Keys(k)`: delegates to `export::keys::eval` - likewise one definition shared with `camera::ease`. *Note:* this is the one arm whose result is deliberately NOT confined to `[0, 1]`; a keyframed curve may anticipate below 0 or overshoot past 1, so the "Returns" line above is a statement about the named curves, not about `Keys`.

**Note on which `ease` actually runs.** Nothing in the crate calls this function today: the live paths (`CameraSim`, `LayoutTrack`, `SpotlightSim`) all use `export::camera::ease`, whose `Smooth` is smoothstep and whose `Spring` really overshoots. The two agree on `Linear`, `EaseIn`, `EaseOut`, `EaseInOut` and now `Cubic`; they differ on `Smooth` and `Spring`. The TS preview mirror (`layoutTrack.ts`) mirrors `camera::ease`, i.e. the one that runs.

### Behaviors worth knowing

- `smooth_hits_endpoints_and_eases_out` (unit test): `ease(Smooth, 0.0)=0.0`, `ease(Smooth, 1.0)=1.0`, and `ease(Smooth, 0.5) > 0.5` (ease-out is past the linear midpoint at t=0.5, reaching 0.875).
- `linear_is_identity_clamped` (unit test): `ease(Linear, 0.3)=0.3`; out-of-range `-1.0` clamps to `0.0`, `2.0` clamps to `1.0`.

### Used by

- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` may call `ease` for region-specific curves.
- `src-tauri/src/export/scene/layout.rs` - `LayoutTrack` uses it for cross-dissolve transition easing.
