# src-tauri/src/export/camera/smoothing.rs

The opt-in output filter for `CameraSim::step`: a **critically damped second-order** (spring-damper) pass over the pose the step just produced, with one knob - `ZoomConfig::smoothing_ms`. Causal and per-step, so it works identically in the export loop and in the editor preview's `camera_track`, and 0 returns the camera untouched, so the shipped trajectory is unchanged unless a caller opts in.

**Why a post-pass and not a change to the sim.** The jank probe (`jank_probe_tests` in `mod.md`) showed the roughness was dominated by *target discontinuities* - the aim function switching between phases, a handoff blend restarting from zero velocity, the centre freezing for the zoom-out, the in-frame clamp truncating a ramp - not by noise in a single phase. A second-order filter on the OUTPUT spreads any such step over its settle time without touching the plant, so `smoothing_ms: 0` is a literal identity and no existing behaviour can regress through a changed feedback loop. The cost is that it can only *smear* a discontinuity, never remove its cause - and that is why the first four of those causes were subsequently fixed IN the sim (`follow.md`, `handoff.md`), which took the scene's worst spike from 129.8 to 6.7 screen px/frame^2 with the filter still off. What is left at the top of the probe's table is the in-frame clamp (H6), still open; the filter remains the opt-in way to smear it.

**Why critically damped.** Underdamped (a normal spring) overshoots, which on a camera reads as a bounce at the end of every zoom; overdamped is a slow first-order-ish crawl. Critical damping is the fastest approach with no overshoot, which is what "the camera never wobbles, it just arrives" means numerically.

## SETTLE

```rust
const SETTLE: f32 = 5.0;
```

The dimensionless product `w * smoothing_ms` (in seconds) the filter is tuned to. A critically damped step response is `1 - (1 + w t) e^(-w t)`, which at `w t = 5` has covered ~96% of the step, so `smoothing_ms` reads as "time to settle" rather than as an opaque stiffness. `w = SETTLE / (smoothing_ms / 1000)`.

## Chan

```rust
#[derive(Clone, Copy, Default)]
struct Chan { y: f32, v: f32 }
```

One filtered scalar channel - `CameraSim` keeps three (scale, cx, cy), each filtered independently on the same `w`.

- `y: f32` - the filtered value, i.e. what is returned. *Why stored:* the filter is causal and stateful; each step continues from the previous output, not from the raw input.
- `v: f32` - its velocity. *Why it exists:* carrying velocity is what lets the filtered output pass through a target step without a corner. (It is also precisely what the handoff `Transition` used to lack - H2 in the probe - and the fix there borrowed the idea: `handoff.md`.)

## Chan::step

```rust
fn step(&mut self, x: f32, w: f32, dt: f32) -> f32
```

Advances the channel toward `x` by `dt` seconds and returns the new `y`.

### Inputs

- `x: f32` - the target for this step (the sim's raw output). *Why:* held constant across the step, which is exact here - the raw pose is only defined at sample points anyway.
- `w: f32` - the undamped natural frequency, `SETTLE / smoothing_s`. *Why:* the single stiffness both the position and velocity terms share, since critical damping fixes the damping ratio at 1.
- `dt: f32` - the real step length in **seconds**, from `apply`'s `dt_ms`. *Why real, not assumed:* the exporter's rate is settings-resolved (60 or 30fps), so baking in 1/60 would reproduce exactly the per-step-vs-per-millisecond flaw the probe measures in the cursor low-pass (H4) and used to measure in the follow lerp.

### Returns

The new filtered value. The update is the **analytic** critically damped solution - with `d = y - x` and `b = v + w*d`, `y(t) = x + (d + b t) e^(-w t)` and `v(t) = (v - w b t) e^(-w t)` - so it is unconditionally stable at any `w * dt`. That matters in practice: a 120ms setting at 60fps is already `w * dt = 0.69`, well past the ~0.5 where the explicit-Euler spring rings and then diverges.

## Damped2

```rust
pub(crate) struct Damped2 { s: Chan, x: Chan, y: Chan, on: bool }
```

The whole filter's state, owned by `CameraSim`. Reset comes for free: `FrameRenderer::reset_camera` rebuilds the `CameraSim`, so a preview scrub re-primes the filter along with everything else.

- `s, x, y: Chan` - the scale, cx and cy channels.
- `on: bool` - whether the filter is primed. *Why a flag at all:* turning the knob off clears it, so turning it back on mid-clip primes from the live pose instead of continuing from a stale one. There is no stored clock - the step length arrives as an argument, so the filter cannot disagree with the rest of the sim about how long a frame is.

## Damped2::new

```rust
pub(crate) fn new() -> Self
```

An un-primed filter (all channels zero, no clock). It cannot be "started at the frame centre" by construction, because the first call with smoothing enabled primes from the live pose instead.

## Damped2::apply

```rust
pub(crate) fn apply(&mut self, dt_ms: f32, cam: Camera, ms: u32, fw: f32, fh: f32) -> Camera
```

Filters one camera pose and re-clamps the result into the frame.

### Inputs

- `dt_ms: f32` - the caller's EXACT frame period, handed down from the frame loop through `FrameRenderer::step_camera` and `CameraSim::step` (which clamps it once). *Why not a timestamp difference, as this took before:* frame timestamps are whole milliseconds, so differencing them reads 16/17/17/16 at 60fps - a rounding artifact, not a real timing difference - and a second-order filter turns that straight into a per-frame ripple.
- `cam: Camera` - the pose `CameraSim::step` just computed (already clamped in-frame). *Why after the clamp:* the raw signal is then guaranteed in-frame, so the filter only ever tracks legal poses and its own output needs a clamp for safety, not for correction.
- `ms: u32` - `ZoomConfig::smoothing_ms`. **0 returns `cam` by reference-identity of value** - no arithmetic, no state - which is what makes "off" bit-identical rather than merely close.
- `fw: f32, fh: f32` - frame dimensions, for the re-clamp.

### Returns

The filtered `Camera`, re-clamped with `half = frame / (2 * scale)` exactly as `CameraSim::step` does. The first primed call returns `cam` unchanged (state seeded to it with zero velocity), so enabling smoothing never introduces a startup slide.

### Behaviors worth knowing

- `smoothing_off_is_bit_identical` (unit test, `jank_probe_tests.rs`'s `mod filter`): an FNV-1a fingerprint of all 721 `(scale, cx, cy)` samples of the probe scene equals the value captured from the build before this module existed.
- `smoothing_before_after` (unit test): on the 12s probe scene, whole-run jerk rms (screen px/frame^2) `cx 3.50 -> 2.57 -> 2.10` and max `31.7 -> 15.8 -> 13.2` for `smoothing_ms` 0 / 120 / 250; the lag paid is a best-fit 33ms (peak-velocity delay 33ms, peak position error 118px during a 1400px/400ms sweep) at 120, and 83ms (50ms, 260px) at 250. (Before the H1/H2/H5/H4 sim fixes the same row read `cx 6.01 -> 3.41 -> 2.53`, max `129.8 -> 36.6 -> 18.0` - most of what the filter used to be buying has since been removed at the source.)
- `smoothing_tames_the_worst_spike_but_cannot_remove_it` (unit test): the 9200..9700ms window around R3's ramp-in -> hold boundary shrinks monotonically - -18% at 60ms, -39% at 120, -62% at 250, -75% at 400 - but never vanishes, because a causal filter can only spread a step over its settle time. That window was `cx 129.8` before `follow::aim` removed the aim discontinuity itself and is `6.7` now, with no filter at all: the cure for a discontinuity is its cause, not a post-pass.
