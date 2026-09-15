# src-tauri/src/export/cursor/draw/tilt.rs

The cursor's **motion tilt**: the very slight lean a pointer takes on when it is thrown across the screen, and the single overshoot it corrects through when it comes to rest. Other editors do this; a recorded pointer never does it by itself, because a mouse cursor has no orientation to lag behind its own motion - which is exactly why adding it reads as "hand-animated" rather than "captured" (the fake-polish north star).

Two filters in series:

1. a **low-pass on the smoothed cursor's own velocity**, so the lean follows the gesture rather than one stray sample;
2. a **lightly under-damped spring** chasing the angle that velocity asks for. The spring is what puts the overshoot in: cut its target to zero and it swings past once, by about a degree, roughly 200 ms after the stop, then settles.

### Why these specific spring numbers

The spring's damping ratio `Z = 0.5` and natural frequency `W = 18` rad/s overshoot by about 16% of the lean it is returning from (about a degree from a full 6-degree lean), with that peak landing at `pi / (W * sqrt(1 - Z^2))`, about 201 ms after the stop. The next swing back is about 0.16 degrees, under a tenth of a pixel at the far corner of a rendered cursor - one visible overshoot, then settled, rather than a series of shrinking wobbles a viewer would notice.

Both run on a **fixed 1 ms substep grid** with the leftover carried, which is what makes a 30 fps export match a 60 fps one at the same instant. A first-order low-pass can be re-based to any step exactly (`export::camera::follow::damping` does it in closed form, which is how `smoothness` means one time constant at every output rate); a second-order spring chasing a moving target cannot, so the step is made small and constant instead.

Everything is `f64`, for the same reason `busy.rs` is: `src/editor/stage/cursor/cursorTilt.ts` is the live preview's mirror of this file and has no `f32`. The two are pinned against the same five instants (`tilt_tests.rs`'s `the_five_pins_the_typescript_mirror_must_match` and `cursorTilt.test.ts`'s "matches the five pins"), because the preview swaps in the backend's own frame when the playhead stops - and a cursor that leaned differently a moment ago would visibly twitch at that swap.

## REF_W

```rust
pub const REF_W: f64 = 1920.0;
```

The screen width every tilt speed is measured against. Speeds go through this before they reach the filter, so one gesture leans the same whether it was recorded at 1080p or at 4K - a raw px/ms threshold would mean "twice as fast" on the bigger capture for exactly the same hand movement.

It is also the unit `src/editor/stage/cursor/cursorTilt.ts` works in, which is what lets the preview mirror the filter without ever knowing the recording's pixel size: `CamSample.curx` is already a 0..1 fraction of the screen content, so multiplying by `REF_W` is the whole conversion.

## MAX_DEG

```rust
pub const MAX_DEG: f32 = 6.0;
```

The lean a full-speed sweep **settles** at when `CursorSettings::tilt` is 1, in degrees; `tilt` scales it (see `max_deg`). The cap is applied to the spring's *target*, so the spring still swings ~16% past it on the way in - that swing is the effect, not a leak, and it is the only thing that ever exceeds this number. `a_fast_throw_leans_and_is_capped_by_the_setting` pins both halves: nothing passes `max * 1.2` at any speed, and a sweep held at speed settles *on* the cap.

## Tilt

```rust
#[derive(Default)]
pub struct Tilt {
    vx: f64, vy: f64,
    angle: f64, avel: f64,
    px: f64, py: f64,
    acc: f64,
    primed: bool,
}
```

The lean filter's state. One per `Cursor` (which owns it and feeds it the path it just smoothed).

- `vx`, `vy` - the smoothed velocity, in reference px per ms. *Why smoothed and not the raw frame-to-frame delta:* recorded mouse samples are throttled, so a per-frame difference steps between "several pixels" and "none" even inside one continuous gesture; leaning straight off that would jitter the sprite.
- `angle`, `avel` - the spring's position (degrees, clockwise-positive) and velocity (degrees per second). *Why a spring and not another low-pass:* a first-order filter can only approach its target, never pass it, and the overshoot on stopping is the whole point of the effect.
- `px`, `py` - the previous position, in reference px, to difference against.
- `acc` - elapsed ms not yet integrated; always less than `SUB_MS` after a step. *Why carried rather than dropped:* dropping it would make the filter run slightly slow, and differently slow at each output rate - the exact thing the fixed grid exists to prevent.
- `primed` - whether there is a previous position to difference against. The first call only records one.

### Used by

- `src-tauri/src/export/cursor/mod.rs` - `Cursor` owns one, steps it inside `at`, and exposes the result through `Cursor::tilt_deg`.

## Tilt::new

```rust
pub fn new() -> Self
```

An upright, unprimed filter (all state zero). Called once by `Cursor::new`.

## Tilt::reset

```rust
pub fn reset(&mut self)
```

Back to upright and unprimed. Called by `Cursor::reset` (and so by `FrameRenderer::snap_cursor`) at a cut and at a preview rewind, so the cursor never arrives at the far side of a splice still leaning from the gesture before it - the same reason the smoothed position itself is dropped there. Also called by `Cursor::set_tilt(0.0)`, so a slider dragged through 0 cannot leave the sprite frozen mid-lean.

## Tilt::angle_deg

```rust
pub fn angle_deg(&self) -> f32
```

The current lean in degrees, clockwise-positive - the sense `cursorxform::blit_transformed` rotates in, so the value goes straight into a `BusyPose`'s `angle_deg` with no sign flip.

## Tilt::step

```rust
pub fn step(&mut self, x: f32, y: f32, dt_ms: f32, max_deg: f32) -> f32
```

Advance by one frame and return the new lean.

### Inputs

- `x`, `y` - the cursor position in **reference** px (frame px times `ref_scale`). *Why the caller converts rather than passing the screen size in:* the preview's mirror has no screen size to pass, only fractions, and both must land in the same unit or the same gesture would lean differently in the two.
- `dt_ms` - the caller's exact frame period (`render::OUT_STEP_MS` in the export; the tick's own elapsed clip time in the preview). Non-positive means "no time passed" - a dirty-redraw tick while paused - and is handled like the first call: the position is recorded, nothing is integrated. *Why record it anyway:* the next real frame must difference against where the cursor actually is, not a stale point, or a scrub would read as a huge instantaneous velocity.
- `max_deg` - the cap this frame's `tilt` setting allows (`max_deg()`). Passing 0 pins the target at 0, so the spring **unwinds** rather than freezing mid-lean.

### Returns

`f32` - the new lean in degrees, the same value `angle_deg` returns afterwards.

### Implementation

1. First call (or `dt_ms <= 0`): record the position, mark primed, return the unchanged angle.
2. Instantaneous velocity `(x - px) / dt`, held constant across this frame's substeps.
3. `acc += dt`, clamped to `MAX_CATCHUP_MS` (100). *Why a clamp:* only the live preview can hand this a big `dt` (a stalled tab, a scrub); without it a ten-second stall would run ten thousand substeps on one frame.
4. While `acc >= SUB_MS`: low-pass the velocity toward the instantaneous one by `VEL_A` re-based to a 1 ms step, compute `target_deg`, then advance the spring by **semi-implicit Euler** (velocity first, then the angle integrating the *new* velocity). *Why semi-implicit:* the explicit form gains energy step over step on a lightly damped spring - the lean would grow instead of settling.

### Behaviors

- `a_fast_throw_leans_and_is_capped_by_the_setting` - a 2 px/ms sweep leans and settles; nothing passes `max * 1.2` at any speed or setting; a held sweep settles exactly on the cap; the same gesture leans less at `tilt` 0.35 than at 1.
- `stopping_returns_to_upright_through_exactly_one_overshoot` - after the cursor is put down there is exactly one local extremum past upright above 0.2 deg, it stays under 1.5 deg, it peaks 150-300 ms after the stop, and the angle is back under 0.05 deg by 600 ms later.
- `ordinary_pointing_stays_upright` - nothing under the dead zone leans at all, and just past it the lean is a fraction of a degree rather than a jump to the cap.
- `a_vertical_throw_leans_at_half_weight` - a straight-down throw leans half as far as the same speed sideways.
- `the_angle_is_the_same_at_30fps_as_at_60fps` - equal to 1e-3 at every 100 ms mark (a whole number of frames at both rates).
- `tilt_zero_never_leans`, `a_frame_with_no_elapsed_time_only_records_the_position`, `the_five_pins_the_typescript_mirror_must_match`.

## target_deg

```rust
fn target_deg(vx: f64, vy: f64, max: f64) -> f64
```

The angle a velocity of `(vx, vy)` reference px/ms asks for, capped at `max`.

The sign follows the **horizontal** direction of travel - moving right leans the sprite clockwise, left anticlockwise - because that is the lean a held object takes on when it is swung sideways. A purely vertical throw has no such direction, so it leans by its vertical component at **half weight** rather than not at all: `K * (vx + 0.5 * vy)`.

`gate = (speed - DEAD) / speed` fades the lean in from the dead-zone edge instead of switching it on. *Why:* a cursor drifting right at the threshold would otherwise flicker between upright and leaning on alternate frames.

### Returns

`f64` degrees, clamped to `[-max, max]`; exactly `0.0` at or below `DEAD` (0.4 reference px/ms, about 400 px/s at 1080p - so ordinary pointing, reaching for a button or nudging a slider, stays perfectly upright).

## max_deg

```rust
pub fn max_deg(tilt: f32) -> f32
```

The cap the `tilt` setting (0..1, `CursorSettings::tilt`) allows: `MAX_DEG * tilt.clamp(0.0, 1.0)`. The setting is clamped here rather than trusted, so a hand-edited `edit.json` cannot ask for a cartwheel. 0 is off outright - `Cursor::at` then skips the filter entirely, so the angle stays 0 and nothing is integrated.

## ref_scale

```rust
pub fn ref_scale(screen_w: u32) -> f32
```

Frame px -> reference px for a capture `screen_w` wide: `REF_W / screen_w.max(1)`. The `max(1)` is the only guard a degenerate `ScreenInfo` needs; the result is 1.0 at 1080p and 0.5 at 4K.
