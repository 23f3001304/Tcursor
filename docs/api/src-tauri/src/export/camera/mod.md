# src-tauri/src/export/camera/mod.rs

Stateful virtual camera that follows a deterministic eased scale curve fully contained within each zoom region: scale ramps 1 -> target over `zoom_in_ms`, holds, then ramps target -> 1 over `zoom_out_ms`, reaching exactly 1.0 at `end_ms`. Produces a `Camera` value each frame consumed by the compositors and FX overlay. When regions overlap the **highest `layer`** wins (ties broken by the later array index); when the winner changes, a `Transition` eases the output from wherever the camera actually is toward the new winner's steady pose.

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
pub fn static_cam_pose(rect: RectF, ow: f32, oh: f32) -> CamPose
```

Converts the camera panel's un-overridden (layout-resolved) rect into a `CamPose` - the inverse of `rect_from_center` (`export/scene/mod.rs`), i.e. "what the webcam would show with zero `camera_moves`". This is `CameraMoveTrack::sample`'s `live` argument: recomputed every frame from that frame's own scene, it is the pose the keyframe track eases OUT of entering its span and back INTO leaving it (Task 27), so an exit blend follows a layout cross-fade that is still in flight.

### Inputs

- `rect: RectF` - the RESOLVED camera panel's rect (output pixels), taken BEFORE any `camera_moves` override is applied. *Why before:* the override replaces `scene.camera` itself, so this must be read from `scene.camera.rect` right after `track.scene_at`/`shrink_camera` resolve it, and before `override_camera` runs.
- `ow: f32`, `oh: f32` - output frame dimensions in pixels. *Why:* same basis `rect_from_center` converts a `CamPose` back into a rect with, so the two are exact inverses.

### Returns

`CamPose { x: (rect.x + rect.w * 0.5) / ow, y: (rect.y + rect.h * 0.5) / oh, size: rect.h / oh }` - the rect's center as fractions of `ow`/`oh`, and its height as a fraction of `oh` (mirrors `rect_from_center`'s `size * oh = h`).

### Used by

- `src-tauri/src/export/render/mod.rs` - `step_camera` computes this from the pre-override `scene.camera.rect` and passes it to `CameraMoveTrack::sample` as the implicit start pose.

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

## Transition

```rust
struct Transition { from_scale: f32, from_cx: f32, from_cy: f32, start_ms: u32, dur_ms: u32, easing: Easing }
```

One in-flight handoff blend between the region that was previously in control and the new winner.

- `from_scale, from_cx, from_cy: f32` - the camera's actual pose at the instant the winner changed. *Why:* the blend must start from where the picture really is, not from frame-center/scale-1, or the camera visibly snaps.
- `start_ms: u32, dur_ms: u32` - when the handoff began and how long it runs (at least 1). *Why:* for a handoff INTO a region, `dur_ms` is what is LEFT of that region's zoom-in window (`CameraSim::remaining_zoom_in`), so the blend and the region's own ramp end at the same instant and the curve is continuous; for a handoff to `None` it is the outgoing driver's `zoom_out_ms`.
- `easing: Easing` - the entering (or, when exiting to `None`, the outgoing) region's own curve.

## CameraSim

```rust
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32,
    driver: Option<usize>, driver_zoom_out_ms: u32, driver_easing: Easing, transition: Option<Transition> }
```

The virtual camera's mutable state.

- `frame_w: u32, frame_h: u32` - output frame dimensions. *Why:* used to compute the dead-band radius in the hold phase and to clamp the center so the viewport stays in-frame.
- `cx: f32, cy: f32` - current camera center in output pixels. *Why:* carried between frames; eased directly during zoom-in, panned during hold, and held (then reclamped) during zoom-out.
- `scale: f32` - current zoom multiplier. *Why:* carried between frames but recomputed deterministically from `t_ms` each call, not damped from the prior value.
- `driver: Option<usize>` - the winning region's index as of the previous step. *Why:* a change is what triggers a `Transition`; comparing indices (not poses) is how a handoff is detected.
- `driver_zoom_out_ms: u32, driver_easing: Easing` - the current driver's own exit settings, refreshed every step it drives. *Why:* a handoff to `None` must build its exit blend without re-indexing into the CURRENT call's `regions`, which may be a shorter/rebuilt slice (callers legitimately pass `&[]` once a region has elapsed).
- `transition: Option<Transition>` - the in-flight handoff blend, if any. *Why:* while it is `Some` the natural target is read as the winner's STEADY pose (see `CameraSim::step`), and it is cleared both when it completes and when a genuine `None -> Some` fresh start arrives.

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
pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera
```

Advances the simulation one frame at time `t_ms` and returns the resulting `Camera`.

### Inputs

- `t_ms: u32` - milliseconds from the event log origin. *Why:* used to determine which phase (zoom-in, hold, zoom-out) the active region is in.
- `cursor: FramePoint` - current cursor position in panel output pixels (from `coordmap::to_panel`). *Why:* during the hold phase the camera pans only when the cursor exits the dead band, and for a `follow_cursor` region it is ALSO the zoom-in ramp's aim (re-read every step, so the ramp tracks a moving cursor).
- `regions: &[ZoomRegion]` - all zoom regions for this export. *Why:* resolved through `CameraSim::winner`, so the highest-`layer` overlapping region wins (later index breaks ties).
- `cfg: &ZoomConfig` - only `follow_damping` is read. *Why:* it is the per-frame lerp factor panning the center toward the cursor during the hold phase; the hold scale and dead-band radius come from the active region's own `target_scale`, not `cfg` (`cfg.target_scale` is unused here - it only seeds new auto-zoom regions elsewhere).

### Returns

`Camera` with the post-step `cx`, `cy`, and `scale`. The returned value is owned by the caller (exporter) each frame.

### Implementation

1. Find the winner with `CameraSim::winner` (highest `layer`; later index breaks ties).
2. If the winner CHANGED: when there was a previous driver, open a `Transition` from the camera's current pose - `dur_ms = remaining_zoom_in(new_winner, t_ms)` for a handoff into a region, or the outgoing driver's `zoom_out_ms` for a handoff to `None`. When there was NO previous driver (a genuine `None -> Some` fresh start) the natural zoom-in-from-center below is already correct, so instead **clear** any transition still in flight - it is the previous driver's stale exit blend and would otherwise attenuate this region's entire ramp.
3. No active region: snap `scale = 1.0`, `cx/cy` to the frame midpoint.
4. Active region `r`: compute `span = (r.end_ms - r.start_ms).max(1)` and fit the ramps with `fit_durations(r.zoom_in_ms, r.zoom_out_ms, span)` so `zi + zo <= span`.
   - Zoom-in phase (`t_ms < r.start_ms + zi`): `e = ease(r.easing, (t_ms - r.start_ms) / zi)`, **except** while a transition is in flight, where `e = 1` so the natural target is `r`'s steady pose; `scale = 1 + (target_scale - 1) * e`; `cx/cy` eased in lockstep from the frame midpoint toward the aim - `cursor` when `r.follow_cursor`, otherwise `r.anchor`. *Why a live aim:* a cursor-target zoom's stored anchor is screen centre (`fromedit::anchor_for` has no cursor track to resolve against), so easing toward it zoomed into the middle and left the whole pan to the hold phase - the reported "first zooms to a spot, then goes to the cursor". Re-reading `cursor` each step makes the ramp end exactly where the follow phase's own `mx`/`my` clamp window already puts it (the cursor lands inside the dead band, so its first target is the ramp's endpoint), which is what removes the second stage.
   - Zoom-out phase (`t_ms >= r.end_ms - zo`): `e = ease(r.easing, (r.end_ms - t_ms) / zo)`; `scale = 1 + (target_scale - 1) * e`, which is exactly `1.0` when `t_ms == r.end_ms`. Center is left as-is (the in-frame clamp below re-centers it as scale shrinks toward 1).
   - Hold phase (between zoom-in end and zoom-out start): `scale = r.target_scale` (fixed, no easing); compute the dead band `mx = fw / (2*target_scale) * 0.4`, `my = fh / (2*target_scale) * 0.4`, pan the center toward the cursor only when it exits the band, damped by `cfg.follow_damping`.
5. Blend: while a `Transition` is in flight (`elapsed < dur_ms`), lerp `scale`/`cx`/`cy` from `from_*` toward the natural target by `ease(tr.easing, elapsed / dur_ms)`; once it completes, drop it and take the natural target directly. The blended value is written back into `self.*` so the next step's hold-phase cursor-follow damps from the blended pose.
6. Clamp `self.cx` and `self.cy` so the viewport (half-size = `frame / (2 * scale)`) stays within the frame bounds.
7. Return `Camera { cx: self.cx, cy: self.cy, scale: self.scale }`.

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
- `moving_cursor_zoom_lands_on_the_live_cursor_with_no_second_stage` (unit test, `cursor_tests.rs`): with the cursor travelling (960,540) -> (1400,760) across a 350ms ramp in a 1920x1080 frame, the ramp is monotonic (no reversal, no overshoot past its landing) and its endpoint equals the hold phase's first clamp-window target within 2px; with the cursor then parked, the next 640ms drift under 2px. Before the live aim the camera sat motionless at the anchor (960,540) for the entire ramp and only then slid 960 -> 1208 over the following ~700ms - the reported two-stage move.
- `retimed_cursor_zoom_ignores_the_stale_anchor` (unit test, `cursor_tests.rs`): the same region retimed +2s, run twice with anchors 1400px apart, produces bit-identical trajectories (1e-6) and lands on the cursor's position at the NEW start - the stored anchor is not read at all.
- `handoff_into_a_cursor_zoom_blends_toward_the_live_cursor` (unit test, `cursor_tests.rs`): a layer handoff INTO a `follow_cursor` region blends toward the live cursor, since the steady pose the transition eases to is itself computed from `cursor` - the Task 9 handoff rules and the live aim compose.

## autozoom

Generates automatic zoom regions from recorded mouse clicks and optional typing events. Key items: `generate(events, screen, cfg, typing, smart)` - returns `Vec<ZoomRegion>` with one region per detected activity burst; `extend_hold` - private helper that chains consecutive activities within an idle gap.

## manual

Converts `ZoomHoldStart`/`ZoomHoldEnd` action pairs into `ZoomRegion` values anchored at the cursor position when the key was pressed. Key items: `from_actions(actions, events, screen, cfg) -> Vec<ZoomRegion>`.

## moves

Keyframed webcam PiP position+size interpolator, the export source of truth for `EditDoc.camera_moves`. Key items: `CameraMoveTrack::from_doc(&[CameraMove]) -> Self`, `CameraMoveTrack::sample(t_ms) -> Option<CamPose>`.
