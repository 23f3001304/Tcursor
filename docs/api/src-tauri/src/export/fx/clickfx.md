# src-tauri/src/export/clickfx.rs

Computes the set of live click effect hits at a given playback time, plus pure math helpers for ripple radius and fade opacity. This is the data layer for click effects: it produces `Hit` values from the event log; `clickdraw` consumes them to produce pixels. All functions are pure and stateless.

## Hit

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit { pub sx: i32, pub sy: i32, pub progress: f32 }
```

A single active click effect at query time.

- `sx`, `sy` - raw screen coordinates of the `MouseDown` event in recording space (not output pixels). *Why recording space:* the exporter maps `(sx, sy)` through the current scene and zoom camera to output pixels before passing them to `clickdraw`; keeping them in recording space decouples this module from the camera.
- `progress` - `(et - click.t) / life_ms` clamped to `[0, 1]`. *Why:* a single normalised progress value drives both the radius (grows with progress) and the fade alpha (falls with progress), so `clickdraw` never needs to re-compute timing.

### Used by

- `src-tauri/src/export/fx_state.rs` - `FxState::build` calls `hits_at` to populate `FxState.hits` from the event log; `FxHit` in `fx_state` is the projected counterpart (output-space x/y, same progress).
- `src-tauri/src/export/clickdraw.rs` - consumes `FxState.hits` (which are based on `Hit`) and calls `ripple_radius` and `fade_alpha` to compute per-pixel draw parameters.

## hits_at

```rust
pub fn hits_at(events: &[MouseEvent], et: u32, life_ms: u32) -> Vec<Hit>
```

Returns all `Hit` values for `MouseDown` events alive at event-time `et`.

### Inputs

- `events: &[MouseEvent]` - the full mouse event log. *Why:* only `EventKind::Down` entries are used; the filter skips moves and ups so only deliberate clicks produce effects.
- `et: u32` - current frame event-time in milliseconds. *Why:* defines the observation window; a click at time `c` is alive when `et >= c && et - c < life_ms`.
- `life_ms: u32` - lifetime of each click effect in milliseconds; clamped to minimum 1 internally to prevent division-by-zero in `progress`. *Why caller-supplied:* the effect duration is a user-configurable setting; different FX styles or future variants may use different lifetimes.

### Returns

`Vec<Hit>` in the order of `events` (oldest click first, newest last). Future clicks (`et < c`) and expired clicks (`et - c >= life_ms`) are excluded. Returns an empty vector when no clicks are alive.

### Behaviors

- `only_downs_within_lifetime_are_active` - at t=300 with life=600, two `Down` events at t=0 and t=100 both appear; a `Move` event is ignored; the older click has the larger `progress`.
- `expired_and_future_clicks_are_excluded` - at t=700 with life=600, a click at t=0 (elapsed=700 >= 600, expired) and t=1000 (future) both produce no `Hit`.

## ripple_radius

```rust
pub fn ripple_radius(progress: f32, r_max: f32) -> f32
```

Linear ring radius (in output pixels) at `progress`, growing from 0 to `r_max`.

### Inputs

- `progress: f32` - normalised effect age 0..1. *Why clamped:* `progress.clamp(0, 1)` guards against floating-point overshoot near lifetime boundaries.
- `r_max: f32` - maximum radius in output pixels. *Why passed in:* the caller in `clickdraw::draw_clicks` derives `r_max = oh * 0.06` so the ring scales with resolution; keeping the formula here pure makes it independently testable.

### Returns

`f32`. 0.0 at `progress = 0`; `r_max` at `progress = 1`.

### Behaviors

- `radius_grows_and_alpha_fades` - `ripple_radius(0.0, 100.0) ~= 0`; `ripple_radius(1.0, 100.0) ~= 100`.

## fade_alpha

```rust
pub fn fade_alpha(progress: f32, intensity: f32) -> f32
```

Effect opacity at `progress`, scaled by the user's intensity setting.

### Inputs

- `progress: f32` - normalised effect age 0..1. *Why:* opacity falls linearly from `intensity` at `progress = 0` to 0 at `progress = 1` so effects naturally vanish without an abrupt cut.
- `intensity: f32` - user intensity setting in 0..1. *Why:* allows the user to dim all click effects globally without changing their timing or geometry.

### Returns

`f32` in `[0, 1]`. Equal to `(1 - progress) * intensity`. At `progress = 0` (instant of click) returns `intensity`; at `progress = 1` returns 0.

### Behaviors

- `radius_grows_and_alpha_fades` - `fade_alpha(0.0, 1.0) > fade_alpha(0.9, 1.0)`; `fade_alpha(0.5, 0.5) ~= 0.25`.
