# src-tauri/src/export/fx/click/clickfx.rs

Computes the set of live click effect hits at a given playback time, plus the two timing curves EVERY click style shares. This is the data layer for click effects: it produces `Hit` values from the event log; `clickdraw` consumes them to produce pixels. All functions are pure and stateless.

**The timing curves are a three-way mirror.** `ease_out` and `fade_alpha` are the Rust half of `fx_clicks.wgsl`'s `fx_ease`/`fx_alpha` (the shader is the reference look) and of `src/editor/stage/fx/ripplePreview.ts`'s `easeOut`/`rippleAlpha` (the editor preview). All three pin the same five sample points in their own tests, so a drift in any one of them fails somewhere.

## Hit

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit { pub sx: i32, pub sy: i32, pub progress: f32 }
```

A single active click effect at query time.

- `sx`, `sy` - raw screen coordinates of the `MouseDown` event in recording space (not output pixels). *Why recording space:* the exporter maps `(sx, sy)` through the current scene and zoom camera to output pixels before passing them to `clickdraw`; keeping them in recording space decouples this module from the camera.
- `progress` - `(et - click.t) / life_ms` clamped to `[0, 1]`. *Why:* a single normalised progress value drives both the radius (grows with progress) and the fade alpha (falls with progress), so `clickdraw` never needs to re-compute timing.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `FxState::build` calls `hits_at` to populate `FxState.hits` from the event log; `FxHit` in `fx_state` is the projected counterpart (output-space x/y, same progress).
- `src-tauri/src/export/fx/click/clickdraw.rs` - consumes `FxState.hits` (which are based on `Hit`) and calls `ripple_radius` and `fade_alpha` to compute per-pixel draw parameters.

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

## ease_out

```rust
pub fn ease_out(progress: f32) -> f32
```

The shared radius easing for every click style: ease-out cubic, `1 - (1 - p)^3`.

### Inputs

- `progress: f32` - normalised effect age 0..1, clamped internally so a boundary overshoot cannot push a radius past its maximum.

### Returns

`f32` in `[0, 1]`. 0 at `progress = 0`, 1 at `progress = 1`, and already 0.875 at the halfway point.

*Why not linear:* a radius that grows at a constant rate reads as a widget animating on a timer. An eased one leaves the click fast and settles, which is what an impact looks like. Every radius in `fx_clicks.wgsl` runs through this, including Shockwave's refraction band - the band and the ring it carries have to ease together or the glass separates from its own highlight.

### Behaviors

- `ease_out_is_pinned_at_five_points` - 0 / 0.578125 / 0.875 / 0.984375 / 1 at p = 0, 0.25, 0.5, 0.75, 1. The identical five are pinned in `ripplePreview.test.ts`.
- `ease_out_clamps_outside_the_life` - `ease_out(-1) == 0`, `ease_out(2) == 1`.

## smoothstep

```rust
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32
```

The GPU builtin (`t * t * (3 - 2t)` over the clamped `(x - e0) / (e1 - e0)`), available to the CPU renderer so `clickdraw.rs` can reproduce the shader's feathers, flash fade and Pulse core fade with the same curve rather than an approximation of it.

### Used by

- `src-tauri/src/export/fx/click/clickfx.rs` - `fade_alpha`'s release tail.
- `src-tauri/src/export/fx/click/clickdraw.rs` - `flash`'s 80 ms fade, `soft_disc`'s feather, Pulse's white-core fade.

## ripple_radius

```rust
pub fn ripple_radius(progress: f32, r_max: f32) -> f32
```

Ring radius (in output pixels) at `progress`, easing out to `r_max` - `ease_out(progress) * r_max`.

### Inputs

- `progress: f32` - normalised effect age 0..1. For Ripple's 2nd and 3rd rings the caller passes `progress - 0.15` / `progress - 0.30`, their launch offsets, so the same curve produces the trailing rings.
- `r_max: f32` - maximum radius in output pixels. *Why passed in:* `clickdraw::draw_clicks` derives `r_max = oh * 0.06` so the ring scales with resolution; keeping the formula here pure makes it independently testable.

### Returns

`f32`. 0.0 at `progress = 0`; `r_max` at `progress = 1`; 0.875 of `r_max` at the halfway point (eased, not the linear half).

### Behaviors

- `radius_eases_out_to_the_max` - 0 at p=0, `r_max` at p=1, and 87.5 of 100 at p=0.5 (the eased value, pinning that this is not linear any more).

## fade_alpha

```rust
pub fn fade_alpha(progress: f32, intensity: f32) -> f32
```

The shared opacity for every click style: the full `intensity` for the first 55% of the life, then a smoothstep release to 0 - `(1 - smoothstep(0.55, 1, p)) * intensity`.

### Inputs

- `progress: f32` - normalised effect age 0..1, clamped.
- `intensity: f32` - the user's intensity setting in 0..1, clamped. *Why:* dims all click effects globally without changing their timing or geometry.

### Returns

`f32` in `[0, 1]`. `intensity` at `progress <= 0.55`; exactly 0 at `progress = 1`.

*Why not the old linear `1 - p`:* the effect was already half gone at mid-life, so it read as a fade rather than as a hit, and it still had visible alpha on the very last frame of the life, which popped off. Holding then releasing gives the effect a body and lets it end on nothing.

### Behaviors

- `fade_alpha_is_pinned_at_five_points` - 1 / 1 / 1 / 0.5829904 / 0 at p = 0, 0.25, 0.5, 0.75, 1. The identical five are pinned in `ripplePreview.test.ts`.
- `fade_alpha_scales_by_intensity_and_clamps` - `fade_alpha(0.5, 0.5) == 0.5` (inside the hold, so the intensity itself), and both inputs clamp outside 0..1.
