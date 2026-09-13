# src-tauri/src/export/camera/handoff.rs

One in-flight handoff between whichever zoom region was previously "in control" and the new winner (`CameraSim::step` opens one whenever `winner` changes and there WAS a previous driver). It eases the camera from wherever it actually is toward the new winner's own natural target over `dur`, using the entering - or, for a handoff to `None`, the outgoing - region's easing curve. Its clock is the caller's exact step length, accumulated here, not a difference of rounded frame timestamps.

**Why it carries velocity.** The probe's H2 (`jank_input_tests::h2_handoff_restarts_from_zero_velocity`) measured a camera panning at a steady 10 px/frame being handed to a higher-layer region: it stopped dead for a frame and then re-launched (mean |v| 10.00 -> 5.13 px/frame over the 80ms after the seam, with a literal 0.00 at the seam itself). The cause is that a position-only ease inherits the curve's own start slope, and `ease'(0) = 0` for smoothstep - the blend contributes no velocity on its first steps no matter how fast the camera was already moving, and the sim had no velocity term for it to continue. The fix is a cubic-Hermite blend: the same eased position term plus a velocity term that is zero at both ends, so the handoff still starts at the captured pose and still lands **exactly** on the target at `dur_ms` (non-negotiable - the incoming region's own ramp and the blend have to agree about where the camera ends up).

## h10

```rust
fn h10(u: f32) -> f32
```

Cubic-Hermite's velocity basis `u^3 - 2u^2 + u`, written factored as `u * (u - 1)^2`.

### Inputs

- `u: f32` - normalized progress through the blend, `elapsed / dur_ms`.

### Returns

`u(u-1)^2`: **0 at both ends** (so the blend's endpoints are untouched by the velocity term), slope **1 at u=0** (so scaling it by `dur * v0` reproduces the camera's real entry velocity in px/ms) and slope **0 at u=1** (so the blend arrives with the target's own velocity, not a leftover of the old one). Its peak is `4/27 ~ 0.148` at `u = 1/3`, which bounds the velocity term at ~15% of the distance the camera would free-fly over the whole blend.

## Transition

```rust
pub(crate) struct Transition { from: Camera, vel: (f32, f32, f32), t: f32, dur: f32, easing: Easing }
```

- `from: Camera` - the camera's actual pose at the seam. *Why:* the blend must start from where the picture really is, not from frame-center/scale-1, or the camera visibly snaps. Note this is the pose at the PREVIOUS sample (see `Transition::open`).
- `vel: (f32, f32, f32)` - that pose's `(cx, cy, scale)` velocity, **per millisecond**. *Why per ms and not per step:* a per-step velocity would mean different physics at every output rate - the same flaw H4 measures elsewhere.
- `t: f32, dur: f32` - the blend's own clock and length, in ms. *Why its own clock rather than frame timestamps:* `t` accumulates the caller's EXACT step length, so `u = t / dur` advances by the same amount every frame. Deriving it from `t_ms` would reinherit that clock's whole-millisecond rounding (16/17/17/16 at 60fps) and ripple the eased position. `dur` is what is LEFT of the incoming region's zoom-in window (`CameraSim::remaining_zoom_in`) so the blend and the region's own ramp end at the same instant - or, for a handoff to `None`, the outgoing driver's `zoom_out_ms` - plus the one backdated step.
- `easing: Easing` - the entering (or, when exiting to `None`, the outgoing) region's own curve, applied to the POSITION term only.

## Transition::open

```rust
pub(crate) fn open(from: Camera, vel: (f32, f32, f32), dt_ms: f32, dur_ms: f32, easing: Easing) -> Self
```

Builds a blend with its clock at 0 and `dur = dur_ms + dt_ms` (floored at 1, so `blend` can never divide by zero).

*Why `dur` is stretched by one step:* `blend` advances the clock BEFORE sampling, so its first call lands one step in rather than at `u = 0`. That is deliberate - `u = 0` returns `from` exactly, and `from` is the pose the previous step already emitted, so a blend that sampled it would emit that pose a second time: one frame of literally zero motion, which is the stall the velocity carry exists to remove (it showed in the probe as the `0.00 px/f` sample at the seam). The seam is therefore the PREVIOUS sample, and adding that step back to `dur` keeps the blend ending exactly where the incoming region's ramp does.

## Transition::blend

```rust
pub(crate) fn blend(&mut self, dt_ms: f32, target: Camera) -> Option<Camera>
```

Advances the blend by one frame and returns the pose on the way to `target`.

### Inputs

- `dt_ms: f32` - the caller's exact frame period, added to the blend's clock before sampling.
- `target: Camera` - the new winner's natural target for THIS instant, recomputed by the caller every step. *Why a parameter and not stored:* while a blend is in flight the natural target is the winner's STEADY pose, which for a cursor-aimed region moves with the cursor - the blend has to chase it, not a snapshot of it.

### Returns

`Some(pose)` while `t < dur`, where each channel is

```
from + (target - from) * ease(easing, u) + u(u-1)^2 * dur * v0
```

and `None` once the blend has run its course - at which point the caller drops the `Transition` and takes `target` exactly.

### Behaviors worth knowing

- `the_blend_carries_the_captured_velocity_into_its_first_step` (unit test): with the target where the camera already is, a 1ms step equals 1ms of free flight to within 1%; over a real export frame toward a target 800px away the first step is at least a full frame of free flight (position-only easing moved it 0.06px). It also pins the backdating - a blend that re-emitted `from` here would fail.
- `the_blend_still_lands_exactly_on_the_target` (unit test): driven one frame at a time exactly as `CameraSim` drives it, for entry velocities from -2 to +3 px/ms, the blend runs ~`dur` ms, its last frame is within 6px of the target, and it stays finished afterwards.
- `a_moving_camera_is_not_stopped_dead_by_a_handoff` (unit test): the H2 scenario end-to-end through `CameraSim` - mean |v| over the 80ms after the seam stays above 80% of the 140ms before it.
- The two handoff spikes the 12s scene showed at t=5000 (`13.9` scale and `13.7` cx screen px/frame^2, both classified "H2 handoff driver 0 -> 1") drop out of the top-10 table entirely.
