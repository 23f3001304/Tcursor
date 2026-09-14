# src-tauri/src/export/camera/mod.rs

Stateful virtual camera that follows a deterministic eased scale curve fully contained within each zoom region: scale ramps 1 -> target over `zoom_in_ms`, holds, then ramps target -> 1 over `zoom_out_ms`, reaching exactly 1.0 at `end_ms`. Produces a `Camera` value each frame consumed by the compositors and FX overlay. When regions overlap the **highest `layer`** wins (ties broken by the later array index); when the winner changes, a `Transition` (`handoff.md`) eases the output from wherever the camera actually is - and at whatever velocity it has - toward the new winner's steady pose. Every phase aims at one shared point (`follow.md`), which is what keeps the phase boundaries continuous.

## ease

```rust
fn ease(e: Easing, p: f32) -> f32
```

Normalized easing curve mapping progress `p` in `[0,1]` to `[0,1]`.

### Inputs

- `e: Easing` - which curve to apply. *Why:* `Linear` is identity; `Smooth` is smoothstep (`3p^2 - 2p^3`); `Spring { .. }` is ease-out-back, overshooting slightly past 1 near the end before settling (the `stiffness`/`damping` fields are unused - only the variant tag matters); `EaseIn`/`EaseOut`/`EaseInOut` are quadratic accelerate (`p^2`) / decelerate (`p*(2-p)`) / symmetric (`2p^2` then `1-2(1-p)^2`); `Cubic { x1, y1, x2, y2 }` delegates to `export::cubic::eval` for a user-drawn bezier. **This is the `ease` that actually runs** - `CameraSim`, `LayoutTrack` and `SpotlightSim` all call it, and the TS preview mirror (`layoutTrack.ts`) mirrors this one, not `export::easing::ease`.
- `p: f32` - progress, clamped to `[0,1]` before easing. *Why:* callers pass raw fractions that can fall slightly outside range at the boundaries; clamping avoids NaN/overshoot from the cubic terms.

### Returns

The eased progress value, `~0` at `p=0` and `~1` at `p=1` (with `Spring`, and a `Cubic` whose `y` handles overshoot, briefly exceeding 1).

## static_cam_pose

```rust
pub fn static_cam_pose(panel: &Panel, ow: f32, oh: f32) -> CamPose
```

Converts the camera panel's un-overridden (layout-resolved) panel into a `CamPose` - the inverse of `rect_from_center` + `override_camera` (`export/scene/mod.rs`), i.e. "what the webcam would show with zero `camera_moves`". This is `CameraMoveTrack::sample`'s `live` argument: recomputed every frame from that frame's own scene, it is the pose the keyframe track eases OUT of entering its span and back INTO leaving it (Task 27), so an exit blend follows a layout cross-fade that is still in flight - and, since keyframe shapes, the shape a `"layout"` keyframe inherits.

### Inputs

- `panel: &Panel` - the RESOLVED camera panel (output pixels), taken BEFORE any `camera_moves` override is applied. *Why before:* the override replaces `scene.camera` itself, so this must be read from `scene.camera` right after `track.scene_at`/`shrink_camera` resolve it, and before `override_camera` runs. *Why the panel, not just its rect:* the pose now carries the panel's shape too (`radius`).
- `ow: f32`, `oh: f32` - output frame dimensions in pixels. *Why:* same basis `rect_from_center` converts a `CamPose` back into a rect with, so the two are exact inverses.

### Returns

`CamPose { x: (r.x + r.w * 0.5) / ow, y: (r.y + r.h * 0.5) / oh, size: r.h / oh, round: Some(panel.radius / min(r.w, r.h).max(0.001)) }` - the rect's center as fractions of `ow`/`oh`, its height as a fraction of `oh` (mirrors `rect_from_center`'s `size * oh = h`), and its corner radius as a fraction of the short side (mirrors `override_camera`'s `round * min(w, h)`; `0.5` for a circle or a Wide stadium).

### Used by

- `src-tauri/src/export/render/mod.rs` - `step_camera` computes this from the pre-override `scene.camera` and passes it to `CameraMoveTrack::sample` as the live pose.
- `src/editor/stage/cameraMoves.ts` - `liveCamPose` is the TS mirror, so the preview inherits the same shape.

## fit_durations

```rust
pub(crate) fn fit_durations(zi: u32, zo: u32, span: u32) -> (u32, u32)
```

Shrinks `(zi, zo)` proportionally so their sum never exceeds `span`, keeping both ramps inside the region.

`pub(crate)` because **layout segments need exactly the same rule**, and there must be one definition of it. `LayoutTrack` (`scene/layout.md`) applies it to every segment's `(transition_ms, transition_out_ms)` at construction: an entry that outlasts its own segment is the layout equivalent of a zoom that never finishes ramping, and it produced a visible jump at the segment boundary (see `scene/layout.md`'s `from_segs`). Its TS mirror, for the live preview, is `fitDurations` in `src/editor/timeline/layoutTrack.ts`.

### Inputs

- `zi: u32, zo: u32` - requested zoom-in/zoom-out durations in ms. *Why:* a region's configured durations can exceed its own `end_ms - start_ms` span (e.g. a very short region), which would otherwise make the ramps overlap or run past the region.
- `span: u32` - the region's total duration (`end_ms - start_ms`, at least 1). *Why:* the upper bound the two ramps must fit under.

### Returns

`(zi, zo)` unchanged if `zi + zo <= span` (or if both are 0); otherwise both scaled down by the same factor `span / (zi + zo)` so they sum to `span`.

## CameraSim

```rust
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32,
    driver: Option<usize>, driver_zoom_out_ms: u32, driver_easing: Easing,
    transition: Option<handoff::Transition>, vel: (f32, f32, f32), stepped: bool,
    smooth: smoothing::Damped2 }
```

The virtual camera's mutable state.

- `frame_w: u32, frame_h: u32` - output frame dimensions. *Why:* used to clamp the center so the viewport stays in-frame.
- `cx: f32, cy: f32` - current camera center in output pixels. *Why:* carried between frames; eased directly during zoom-in, panned during hold, and held (then reclamped) during zoom-out.
- `scale: f32` - current zoom multiplier. *Why:* carried between frames but recomputed deterministically from `t_ms` each call, not damped from the prior value.
- `driver: Option<usize>` - the winning region's index as of the previous step. *Why:* a change is what triggers a `Transition`; comparing indices (not poses) is how a handoff is detected.
- `driver_zoom_out_ms: u32, driver_easing: Easing` - the current driver's own exit settings, refreshed every step it drives. *Why:* a handoff to `None` must build its exit blend without re-indexing into the CURRENT call's `regions`, which may be a shorter/rebuilt slice (callers legitimately pass `&[]` once a region has elapsed).
- `smooth: smoothing::Damped2` - the opt-in critically damped output filter (`smoothing.md`). *Why a field and not a wrapper:* it is per-sim state that must reset with the sim (`FrameRenderer::reset_camera` rebuilds `CameraSim`, so a preview scrub re-primes it for free). Idle and never primed while `ZoomConfig::smoothing_ms` is 0, which is every shipped path today.
- `transition: Option<handoff::Transition>` - the in-flight handoff blend, if any (`handoff.md`). *Why:* while it is `Some` the natural target is read as the winner's STEADY pose (see `CameraSim::step`), and it is cleared both when it completes and when a genuine `None -> Some` fresh start arrives.
- `vel: (f32, f32, f32)` - the `(cx, cy, scale)` velocity of the step just taken, **per millisecond**, measured after the in-frame clamp. *Why:* it is what a new `Transition` carries across the seam, so a moving camera is not stopped dead by a handoff (H2, `handoff.md`).
- `stepped: bool` - whether there has been a previous step to measure `vel` over. *Why:* on the very first call the "previous" pose is the constructor's frame-centre default, not a place the camera ever was, so its velocity would be fiction.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - one `CameraSim` per export; `step` is called in the frame loop.

## CameraSim::new

```rust
pub fn new(frame_w: u32, frame_h: u32) -> Self
```

Creates a `CameraSim` at scale 1.0, centered at `(frame_w/2, frame_h/2)`.

### Inputs

- `frame_w: u32, frame_h: u32` - output frame size. *Why:* initializes the center to the frame midpoint so the first frame is unzoomed and centered.

### Returns

A new `CameraSim` with `scale=1.0`.

## CameraSim::winner

```rust
fn winner(regions: &[ZoomRegion], t_ms: u32) -> Option<usize>
```

The index of the highest-`layer` region active at `t_ms`, or `None` if none is.

### Inputs

- `regions: &[ZoomRegion]` - all zoom regions for this export. *Why:* filtered to those whose `[start_ms, end_ms]` contains `t_ms`, then reduced by `max_by_key((layer, index))`.
- `t_ms: u32` - the instant being resolved.

### Returns

`Some(index)` of the winner - ties on `layer` broken by the LARGER index, i.e. the most recently added region. Priority is `layer` first, array position only as a tiebreak (`highest_layer_wins_not_most_recent`).

## CameraSim::remaining_zoom_in

```rust
fn remaining_zoom_in(r: &ZoomRegion, t_ms: u32) -> u32
```

How much of `r`'s zoom-in window is still ahead of `t_ms` - the length a handoff blend INTO `r` must run.

### Inputs

- `r: &ZoomRegion` - the incoming winner. *Why:* its zoom-in window is computed exactly as `step`'s natural branch does (`fit_durations(zoom_in_ms, zoom_out_ms, span)`, `zin_end = start_ms + zi`) so the two agree to the millisecond.
- `t_ms: u32` - the instant of the handoff.

### Returns

`zin_end - t_ms` when the handoff lands inside `r`'s zoom-in window, otherwise `r.zoom_in_ms` (the handoff arrived during `r`'s hold or zoom-out, so there is no ramp left to meet). *Why it matters:* using the full `zoom_in_ms` for a mid-window handoff would leave the blend still running after the region's own ramp had settled, producing a second, slower curve on top of the first.

## CameraSim::step

```rust
pub fn step(&mut self, t_ms: u32, dt_ms: f32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera
```

Advances the simulation one frame at time `t_ms` and returns the resulting `Camera`.

### Inputs

- `t_ms: u32` - milliseconds from the event log origin, whole (the frame loops build it as `k * 1000 / out_fps` in integer math). *Why:* used to determine which phase (zoom-in, hold, zoom-out) the active region is in. Everything that LOOKS UP by time reads this rounded value - `winner`, `fit_durations`, the ramps' `ease` progress.
- `dt_ms: f32` - the EXACT frame period (`1000 / out_fps`), from the caller. *Why a separate argument rather than `t_ms` minus the previous `t_ms`:* differencing whole-millisecond timestamps reads 16/17/17/16 at 60fps, which is the clock's rounding and not a real timing difference, and every per-millisecond filter here (`follow::damping`, the handoff's velocity carry and clock, `smoothing::Damped2`) would turn it into a ~3%/frame ripple on tracking speed during a fast pan - measured at `+18%` sweep-only jerk rms on the probe scene. Clamped once, to `[0.1, 1000]`ms, and passed to all three; a non-finite value falls back to `follow::REF_STEP_MS`.
- `cursor: FramePoint` - current cursor position in panel output pixels (from `coordmap::to_panel`). *Why:* it is the sole live input to `follow::aim`, which every phase reads - a `follow_cursor` region aims at the cursor itself (re-read every step, so the ramp tracks a moving cursor); an anchored region ignores it and aims at its anchor.
- `regions: &[ZoomRegion]` - all zoom regions for this export. *Why:* resolved through `CameraSim::winner`, so the highest-`layer` overlapping region wins (later index breaks ties).
- `cfg: &ZoomConfig` - `follow_damping` and `smoothing_ms` are read. *Why:* `follow_damping` is the fraction of the remaining error the centre closes in one 60fps frame, converted to this step's real length by `follow::damping`; the hold scale comes from the active region's own `target_scale`, not `cfg` (`cfg.target_scale` is unused here - it only seeds new auto-zoom regions elsewhere).

### Returns

`Camera` with the post-step `cx`, `cy`, and `scale`. The returned value is owned by the caller (exporter) each frame.

### Implementation

1. Find the winner with `CameraSim::winner` (highest `layer`; later index breaks ties).
2. If the winner CHANGED: when there was a previous driver, open a `Transition` (`handoff.md`) from the camera's current pose AND its current velocity - `dur_ms = remaining_zoom_in(new_winner, t_ms)` for a handoff into a region, or the outgoing driver's `zoom_out_ms` for a handoff to `None`, both backdated by one step so the blend's `u = 0` falls on the previous sample rather than re-emitting it. When there was NO previous driver (a genuine `None -> Some` fresh start) the natural zoom-in-from-center below is already correct, so instead **clear** any transition still in flight - it is the previous driver's stale exit blend and would otherwise attenuate this region's entire ramp.
3. No active region: snap `scale = 1.0`, `cx/cy` to the frame midpoint.
4. Active region `r`: compute `span = (r.end_ms - r.start_ms).max(1)` and fit the ramps with `fit_durations(r.zoom_in_ms, r.zoom_out_ms, span)` so `zi + zo <= span`, then resolve the region's aim ONCE with `follow::aim(r, cursor)` (`follow.md`). *Why once, before the phase split:* every phase uses the same point, which is what makes the phase boundaries continuous - the ramp lands exactly where the hold then holds, so the target cannot teleport when the phase changes (the probe's H1b, the scene's largest spike, was precisely that teleport).
   - Zoom-in phase (`t_ms < r.start_ms + zi`): `e = ease(r.easing, (t_ms - r.start_ms) / zi)`, **except** while a transition is in flight, where `e = 1` so the natural target is `r`'s steady pose; `scale = 1 + (target_scale - 1) * e`; `cx/cy` eased in lockstep from the frame midpoint toward the aim. *Why a live aim:* a cursor-target zoom's stored anchor is screen centre (`fromedit::anchor_for` has no cursor track to resolve against), so easing toward it zoomed into the middle and left the whole pan to the hold phase - the reported "first zooms to a spot, then goes to the cursor". An anchored region's aim is its anchor, wherever the cursor is.
   - Hold AND zoom-out phase (everything at or past `zin_end`) share one centre rule: damp `cx/cy` toward `aim` by `cfg.follow_damping`. The lerp factor is `follow::damping(cfg.follow_damping, dt)` - the setting read as a time constant rather than a per-step fraction, so the export's 16/17ms grid and the preview's flat 16ms follow at the same rate (H4), and at exactly 16.667ms it is the configured value untouched. Only the SCALE differs - `r.target_scale` while holding, and `1 + (target_scale - 1) * ease(r.easing, (r.end_ms - t_ms) / zo)` once `t_ms >= r.end_ms - zo`, which is exactly `1.0` at `end_ms`. *Why the ramp-out follows too:* holding the centre fixed for the whole zoom-out stopped a moving camera dead mid-gesture, and from a CLAMPED pose the in-frame clamp then released as one large reverse sweep as its bound relaxed (H5 in the probe, and the scene's `+19 px/frame^2` spike at t=7050). Letting the centre keep tracking means the clamp relaxes underneath a centre already heading the right way.
5. Blend: `Transition::blend` advances the blend by `dt_ms` and returns the blended pose while one is in flight, `None` once it has run its `dur`, at which point it is dropped and the natural target is taken directly. The blended value is written back into `self.*` so the next step's hold-phase cursor-follow damps from the blended pose - with the scale floored at `1.0` on the way in: a handoff's velocity carry (`h10 * vel`, `handoff.md`) can push a zoom-out that ends at speed (Linear, Spring) below full frame for a few frames, which drew the picture smaller than the output with black to the right and below (`the_camera_never_zooms_out_past_full_frame`).
6. Clamp `self.cx` and `self.cy` so the viewport (half-size = `frame / (2 * scale)`) stays within the frame bounds.
7. Record `self.vel` - the clamped pose minus the previous one, divided by `dt_ms`. *Why after the clamp:* the velocity a handoff carries has to be the velocity the picture actually had, including a frame where the clamp was holding the camera still.
8. Hand that clamped pose, and the same `dt_ms`, to `Damped2::apply` with `cfg.smoothing_ms` and return what comes back. At the default 0 this is the identity - the same `Camera { cx: self.cx, cy: self.cy, scale: self.scale }` this step always returned, bit for bit. Non-zero runs the critically damped post-pass and re-clamps its output (`smoothing.md`). The filter deliberately does **not** feed back into `self.*`: the sim's own state, including the hold phase's dead-zone feedback, keeps evolving exactly as it does with smoothing off, so the knob cannot change the trajectory being filtered - only how sharply the camera is allowed to follow it.

### Behaviors worth knowing

- `no_region_is_full_frame` (unit test): with no regions, `step` at t=0 returns scale=1.0, center at frame midpoint.
- `scale_is_one_at_region_end_and_after` (unit test): scale is exactly 1.0 (within 1e-3) at `t_ms == end_ms` and stays 1.0 for any `t_ms` after the region ends - the zoom-out never spills past the region's bound.
- `scale_reaches_target_during_hold` (unit test): scale equals `target_scale` exactly (within 1e-3) at a hold-phase timestamp, since hold no longer damps toward the target.
- `ramp_in_is_monotonic_and_bounded` (unit test): scale climbs monotonically from 1.0 toward (but not reaching) `target_scale` during the zoom-in ramp.
- `durations_that_exceed_span_are_scaled_to_fit` (unit test): a region whose `zoom_in_ms + zoom_out_ms` exceeds its own span still reaches exactly 1.0 at `end_ms` (via `fit_durations` shrinking both ramps proportionally) and never produces NaN.
- `newer_region_preempts_older_overlap` (unit test): with two overlapping regions, the later-starting one's anchor wins when t is in both.
- `center_clamps_inside_frame` (unit test): an anchor at (0,0) with scale~2 keeps `cx >= 200`, `cy >= 150` (half-viewport of an 800x600 frame).
- `highest_layer_wins_not_most_recent` (unit test): a layer-1 region added BEFORE an overlapping layer-0 region still drives the camera - `layer` is the priority, array position is only a tiebreak.
- `handoff_eases_from_current_camera_state_not_frame_center` / `handoff_reaches_the_new_winners_target_once_its_transition_completes` (unit tests): a layer handoff neither snaps toward frame-center at the start nor stalls short of the incoming region's target once the blend finishes.
- `identical_layer_handoff_is_invisible` (unit test): two regions sharing `target_scale` AND `anchor`, handing off at t=2000, hold scale flat at 2.2 and centre flat to within 5px for the whole 2000..4000 window. Before the steady-pose rule, the natural ramp and the blend both eased on the same clock, giving `s - (s-1)*e*(1-e)` - the camera pulsed OUT to ~1.90 (a ~14% dip) around t=2144 and the centre wobbled 618 -> 579 -> 618.
- `a_fresh_zoom_after_an_exit_ramps_at_its_own_pace` (unit test): a zoom starting shortly after an earlier zoom's exit reaches 2.15 within 32ms of the same region simulated alone. Before the stale-exit clear, the leftover `Some -> None` blend attenuated the new ramp (2.15 reached at 3408ms instead of 3360ms, and every intermediate sample lagged, e.g. 1.09 vs 1.32 at t=3168).
- `static_cursor_zoom_matches_the_fixed_anchor_trajectory` (unit test, `cursor_tests.rs`): with the cursor parked exactly where a fixed anchor sits, a `follow_cursor` region reproduces the fixed-anchor trajectory sample for sample (1e-3) for the whole region. The live aim may only change what happens while the cursor MOVES.
- `moving_cursor_zoom_lands_on_the_live_cursor_with_no_second_stage` (unit test, `cursor_tests.rs`): with the cursor travelling (960,540) -> (1400,760) across a 350ms ramp in a 1920x1080 frame, the ramp is monotonic (no reversal, no overshoot past its landing) and its endpoint equals the hold phase's first target within 2px; with the cursor then parked, the next 640ms are a monotone exponential settle onto it (never reversing, each step inside the `follow_damping` bound, ending within 1px of the cursor and within 20px of where the ramp landed). Before the live aim the camera sat motionless at the anchor (960,540) for the entire ramp and only then slid 960 -> 1208 over the following ~700ms - the reported two-stage move.
- `retimed_cursor_zoom_ignores_the_stale_anchor` (unit test, `cursor_tests.rs`): the same region retimed +2s, run twice with anchors 1400px apart, produces bit-identical trajectories (1e-6) and lands on the cursor's position at the NEW start - the stored anchor is not read at all.
- `handoff_into_a_cursor_zoom_blends_toward_the_live_cursor` (unit test, `cursor_tests.rs`): a layer handoff INTO a `follow_cursor` region blends toward the live cursor, since the steady pose the transition eases to is itself computed from `cursor` - the Task 9 handoff rules and the live aim compose.

## handoff

The in-flight blend between the outgoing driver and the new winner - position AND velocity, so a moving camera is not stopped dead at the seam. Key items: `Transition::open`, `Transition::blend`, `h10`. Full rationale in `handoff.md`.

## follow

The region's aim point - where the camera wants its centre - shared by every phase of `CameraSim::step`, which is what makes the phase boundaries continuous, plus the conversion of `follow_damping` from a per-step fraction into a time constant. Key items: `aim(r, cursor) -> (f32, f32)`, `damping(k_ref, dt_ms) -> f32`, `REF_STEP_MS`. `pub(crate)` for exactly one reason: `export::cursor`'s low-pass reads `damping`/`REF_STEP_MS` too. Full rationale and the measured before/after are in `follow.md`.

## autozoom

Generates automatic zoom regions from recorded mouse clicks and optional typing events. Key items: `generate(events, screen, cfg, typing, smart)` - returns `Vec<ZoomRegion>` with one region per detected activity burst; `extend_hold` - private helper that chains consecutive activities within an idle gap.

## manual

Converts `ZoomHoldStart`/`ZoomHoldEnd` action pairs into `ZoomRegion` values anchored at the cursor position when the key was pressed. Key items: `from_actions(actions, events, screen, cfg) -> Vec<ZoomRegion>`.

## moves

Keyframed webcam PiP position+size interpolator, the export source of truth for `EditDoc.camera_moves`. Key items: `CameraMoveTrack::from_doc(&[CameraMove]) -> Self`, `CameraMoveTrack::sample(t_ms) -> Option<CamPose>`.

## smoothing

The opt-in critically damped post-pass applied to `CameraSim::step`'s output, behind `ZoomConfig::smoothing_ms` (0 = off = bit-identical). Key items: `Damped2::apply(t_ms, cam, ms, fw, fh) -> Camera`. Full rationale, the stability argument and the measured jerk/lag trade-off are in `smoothing.md`.

## jank_probe_tests

Test-only (`#[cfg(test)]`) numeric probe of *how the generated motion feels*, as opposed to the correctness tests above. It drives `Cursor::at` + `CameraSim::step` directly - no renderer, no decode - over a 12-second synthetic scene and prints first/second differences, so "the camera feels janky" can be answered with timestamps instead of opinions. Run it with:

```
cargo test --lib jank -- --nocapture --test-threads=1
```

Its files (each its own module so every file stays under the size limit):

- `jank_scene.rs` (`mod jscene`) - the scenario and the drivers. `events()` builds a hand-made `Vec<MouseEvent>` (8ms samples with deterministic +-2px jitter, two clicks, a 1400px/400ms sweep, a long near-still pause); `regions()` builds R1 `follow_cursor` 2.2 (2600..5000, 350 in / 450 out, hold spanning the sweep), R2 anchored 1.8 (5000..7500, gapless after R1), R3 anchored 2.6 at a corner (9000..12000, so the in-frame clamp bites). `run(grid, cfg, alpha)` samples a whole clip into a `Run`; both `Grid::Export` and `Grid::Preview` walk the exporter's own `k * 1000 / 60` frame index (whose TIMESTAMPS land 16/17ms apart even though the frames are a uniform 16.667ms) and hand `CameraSim::step` the exact `STEP_MS` period - `Grid::Preview` used to be the editor's flat integer 16ms step, until `camera_track` was put on the export's grid. `micro_region`/`drive`/`vel`/`band` are the per-hypothesis probes' helpers.
- `jank_metrics.rs` (`mod jm`) - `series` converts a `Run` into three channels expressed in **screen pixels of motion per frame** (a pan of `d_cx` shows as `scale * d_cx`; a scale step `ds` slides content at the viewport edge by `FW/(2*scale) * ds`), so `cx`, `cy` and `scale` jerk are comparable numbers. `top_spikes` ranks |d velocity| across all three, suppressing a spike's own ringing; `classify` attributes each spike to a hypothesis from the run's recorded driver index, clamp flag and cursor-vs-camera distance. `at` linearly interpolates a run at an arbitrary instant exactly as the editor's `camAt` does between `camera_track` samples.
- `jank_phase_tests.rs` (`mod phase`) - H1a/H1b (the aim function changing at ramp-in -> hold, for a follow and for an anchored region - both fixed, `follow.md`), H5 (ramp-out freezing the centre - fixed), H6 (the clamp truncating a ramp - still open; its probe now parks the cursor ON the anchor, since `follow::aim` would otherwise pull the aim back in-frame and there would be no clamp to measure).
- `jank_input_tests.rs` (`mod input`) - H2 (a handoff `Transition` restarting from velocity 0 - fixed, `handoff.md`), H3 (an anchored region's dead-zone edge as a step input to the follow lerp - retired with the dead zone itself on 2026-09-13: an anchored region no longer reacts to the cursor at all), H4 (a low-pass alpha being per-step rather than per-millisecond - fixed in both the camera's follow lerp and the cursor's low-pass, which now share `follow::damping`; the probe's table prints the closed gap).
- `jank_filter_tests.rs` (`mod filter`) - before/after for the opt-in `ZoomConfig::smoothing_ms` post-pass (`smoothing.md`): the bit-identical pin at 0, the jerk/lag trade-off at 120 and 250, and how far the worst spike can be tamed.

### Behaviors worth knowing

- `jank_probe_tables` prints whole-run and sweep-only jerk plus the top-10 spike table for both grids - which, since `camera_track` started walking the export's own frame index at the exact 16.667ms period, are the SAME grid and print identical numbers. That identity is the point, and `preview_grid_vs_true_60fps` asserts it (max |d cx| 0.000 px, down from 28.70). Before the H1 aim fix the two largest spikes in the scene were both at t=9350 - R3's ramp-in -> hold boundary - at -129.8 and -101.6 screen px/frame^2; they are gone from the table entirely now (`follow.md`), and the top of the table is the in-frame clamp (H6, still open).
- `preview_grid_vs_true_60fps` pins the preview as a stand-in for the export. It used to quantify a gap - median |d cx| 0.02px but max 28.7px (74.5 screen px) at t=9344, faithful everywhere except exactly at the sharp transitions, because both runs are stateful and the preview took 751 steps where the export takes 721. `camera_track` now walks the export's own frame index (`preview_track.md`), so the test asserts the two agree exactly: same 721 samples, max |d cx| below 1e-3 px.
