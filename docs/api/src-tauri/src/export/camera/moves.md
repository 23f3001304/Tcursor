# src-tauri/src/export/camera/moves.rs

Keyframed webcam PiP position+size interpolator (`EditDoc.camera_moves`), the export source of truth for the "Move" camera feature. An empty track is a no-op - the caller keeps whatever static `overlay_for` rect it already has, so a doc with no `camera_moves` composites byte-identically to today. Mirrors `LayoutTrack`'s "ease into the entered keyframe using its own easing" convention (`src-tauri/src/export/scene/layout.rs`).

**Task 27 - keyframes own only their span.** Before Task 27 a caller-supplied `static_pose` acted as an implicit keyframe at `t=0`, which meant `sample` returned `Some` for EVERY frame of the clip as soon as one keyframe existed: layout segments that move or resize the webcam panel were silently stomped for the whole recording. Now the track owns exactly `span()` = `[first - KF_BLEND_MS, last + KF_BLEND_MS]` and returns `None` everywhere else, so the layout track owns the panel outside it exactly as if no keyframes existed. The old `static_pose` parameter became `live`: the layout-resolved pose for THIS frame, re-read every call, which the track eases out of on the way into its span and back into on the way out (an exit blend therefore tracks a layout transition that is still moving - the same principle as `CameraSim`'s handoff). No data-model change: `camera_moves` entries are untouched, no doc version bump, old docs simply gain the corrected behavior.

## CamPose

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32 }
```

One resolved webcam-PiP position+size at a frame time.

- `x` / `y` - *the PiP's center, as a fraction (`0.0`-`1.0`) of the output frame - same units as `CameraMove.x`/`.y`.*
- `size` - *the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.*

### Used by

- `src-tauri/src/export/render/mod.rs` - `step_camera` overrides the scene's camera panel rect from a non-`None` sample (Task 4); also the shape `static_cam_pose` (`export/camera/mod.rs`) returns, passed as `sample`'s per-frame `live` pose.

## KF_BLEND_MS

```rust
pub const KF_BLEND_MS: u32 = 350;
```

The handoff length in milliseconds on EACH side of the keyframe span, between the live layout-resolved pose and the track. A module constant, deliberately **not** a setting: it is the semantic width of "entering/leaving the keyframes' authority", not a look knob, and the TS preview mirror (`KF_BLEND_MS` in `src/editor/stage/cameraMoves.ts`) must stay numerically equal or the preview and the export disagree frame-for-frame.

## CameraMoveTrack

```rust
pub struct CameraMoveTrack { kfs: Vec<Kf> }
```

Holds the sorted keyframes for one export, each with its wire-easing string pre-resolved to `Easing` once at construction (mirrors `LayoutTrack::from_segs` resolving `Easing` up front rather than per-sample).

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer` builds one from `doc.camera_moves` and calls `sample` per output frame.

## CameraMoveTrack::from_doc

```rust
pub fn from_doc(moves: &[CameraMove]) -> Self
```

Builds a track from `EditDoc.camera_moves`.

### Inputs

- `moves: &[CameraMove]` - the doc's keyframe list. *Why cloned + defensively sorted:* callers should already hand these in `t_ms` order (the edit ops keep `doc.camera_moves` sorted), but the track must never trust that invariant from outside its own module.*

### Returns

`CameraMoveTrack` with keyframes cloned out of `moves`, sorted ascending by `t_ms`, and each keyframe's `easing` string resolved to `Easing` via `crate::export::render::fromedit::easing_from` (same helper `LayoutTrack::from_segs` uses for `LayoutSeg.easing`).

### Implementation

1. Map each `CameraMove` to an internal `Kf { t_ms, x, y, size, easing: easing_from(&m.easing, Easing::Smooth) }`.
2. `sort_by_key(|k| k.t_ms)`.

## CameraMoveTrack::span

```rust
pub fn span(&self) -> Option<(u32, u32)>
```

The `(entry, exit)` output-time window the track OWNS: `(first.t_ms - KF_BLEND_MS, last.t_ms + KF_BLEND_MS)`, saturating at `0` so a keyframe closer to the start than one blend simply gets a shorter (but still continuous) ease-in. `None` for an empty track. `sample` is `Some` inside this window (inclusive at both ends) and `None` outside it, so the same call answers "do the keyframes own this frame?" for the renderer's smart-zoom guard.

### Used by

- `CameraMoveTrack::sample` itself (its first check).
- `src/editor/timeline/CameraLane.tsx` draws the same window as its translucent span bar, but computes it TS-side (padding its own drag-adjusted keyframe times by `KF_BLEND_MS`) rather than round-tripping through IPC - so the bar tracks a diamond mid-drag, before anything is committed. The TS `camKfRange` is NOT this function's counterpart: it returns the UNPADDED `[first, last]` keyframe range.

## CameraMoveTrack::sample

```rust
pub fn sample(&self, t_ms: u32, live: Option<CamPose>) -> Option<CamPose>
```

Resolves the PiP pose at `t_ms`, or `None` when the keyframes do not own that frame.

### Inputs

- `t_ms: u32` - video timestamp in milliseconds (OUTPUT time - `t - video_start`, the clock every `EditDoc` region list lives on). *Why:* called once per output frame by the exporter, same cadence as `LayoutTrack::scene_at`.
- `live: Option<CamPose>` - the LIVE layout-resolved PiP pose for THIS frame: `export::camera::static_cam_pose` of the resolved-but-not-yet-overridden `scene.camera.rect`. *Why per-frame, not a one-off static pose:* it is the pose the blends ease to and from, and it moves - a layout cross-fade running during the exit window means the blend must chase where the panel is actually going, not where it was when the exit started. `None` skips both blends (each snaps to the nearest end keyframe); the span rule itself never depends on it, so a caller without a live pose still gets the correct Some/None decision.

### Returns

- `None` when the track is empty, or `t_ms` is outside `span()` - the caller's signal to leave the scene's camera panel exactly as the layout resolved it (the "empty means default" model, same convention `LayoutTrack::scene_at`'s gap-fallback and `layout_segs_from_doc`'s `None` use).
- `Some(CamPose)` inside the span, per the five cases below.

### Implementation

1. `span()` -> `None` (empty track) short-circuits; `t_ms < entry || t_ms > exit` -> `None`. **Case 1.**
2. `t_ms < first.t_ms` - **case 2, the entry blend:** `f = ease(first.easing, (t_ms - entry) / (first.t_ms - entry))`, then `mix(live, pose(first), f)`. The window length is `KF_BLEND_MS` unless clamped at `t=0`. Continuous at both ends: `f = 0` at `entry` gives exactly `live`, `f = 1` at `first` gives exactly the keyframe pose. `live = None` (or a zero-length window) -> the first keyframe's pose.
3. `t_ms > last.t_ms` - **case 4, the exit blend:** `f = ease(last.easing, (t_ms - last.t_ms) / KF_BLEND_MS)`, then `mix(pose(last), live, f)`. Because `live` is this frame's value, the target moves with a running layout transition. `live = None` -> the last keyframe's pose.
4. `t_ms == last.t_ms` -> the last keyframe's pose. Also the single-keyframe instant (`first == last`).
5. **Case 3, in-span interpolation - unchanged math:** find `bi`, the first index whose `t_ms` exceeds the query, giving the straddling pair `a = kfs[bi-1]`, `b = kfs[bi]`. `b.t_ms == a.t_ms` (coincident keyframe times) -> `b`'s pose, guarding the division. Else `f = ease(b.easing, (t_ms - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve. *Why `b`'s easing, not `a`'s:* matches `LayoutTrack::scene_at`, which eases into a newly-entered segment using that segment's own `transition_ms`/`easing`, not the one being left.
6. Every blend and interpolation goes through `mix`, a component-wise lerp of the whole pose (`x`/`y`/`size`). The rect is derived from the RESULT once, by `rect_from_center` via `override_camera`, so the panel's aspect handling applies to a blended pose exactly as it does to a keyframed one.

**Case 5, the single keyframe:** `first == last`, so the panel eases in over `KF_BLEND_MS`, sits on the keyframe pose for that instant, and eases back out. A hold requires two keyframes; that is intended, and is what makes "one stray keyframe" a local bump rather than a clip-long takeover.

### Behaviors worth knowing

In `moves_tests.rs` (in-span math):

- `empty_track_samples_to_none` - no keyframes -> `span()` and `sample` are both `None` at any `t_ms`.
- `midpoint_lerp_with_linear_easing_is_the_exact_mean` - `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- `midpoint_with_smooth_easing_diverges_from_the_linear_mean` - at `t=250` of a `[0,1000]` span (`u=0.25`), the sampled `x`/`size` equal `ease(Easing::Smooth, 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- `coincident_keyframe_times_snap_to_b_without_dividing_by_zero` - two keyframes sharing the same `t_ms` never panics or NaNs; sampling exactly at that time returns one of the two poses, not a lerp.
- `out_of_order_input_is_sorted_defensively` - keyframes passed to `from_doc` in descending `t_ms` order still sample correctly, proving the constructor's own sort (not caller discipline) is what keeps `sample` correct.
- `mid_span_interpolation_is_unchanged_by_the_span_rewrite` - the Task 27 regression guard: keyframes at `2000`/`4000` sampled at `3000` reproduce the pre-change value (`x = 0.40`, pinned literally) computed independently of `sample`, and produce the SAME result with `live = None` and with a wildly different `live` - proving `live` has no influence mid-span.

In `moves_span_tests.rs` (span + blends, all with keyframes at `2000`/`4000`, i.e. span `[1650, 4350]`):

- `span_is_the_keyframe_range_padded_by_one_blend_each_side` - also that a keyframe at `t=100` clamps its entry edge to `0` instead of wrapping the `u32`.
- `outside_the_span_the_layout_owns_the_panel` - `sample(1000, live)` and `sample(5000, live)` are `None`; `1649`/`4351` are `None` while `1650`/`4350` are `Some` (the edges are inside).
- `entry_blend_is_continuous_at_both_ends` - at `first - KF_BLEND_MS` the blend equals `live` within `1e-4`; at `first` it equals the keyframe pose within `1e-4`; the midpoint sits strictly between them on every axis.
- `exit_blend_tracks_a_live_pose_that_is_still_moving` - with `live(t)` sweeping linearly across the exit window, the blend at `last + KF_BLEND_MS` equals `live(last + KF_BLEND_MS)` within `1e-4` (and is >0.5 away from the stale `live(last)`), while `t == last` still returns the keyframe pose exactly.
- `single_keyframe_is_a_bump_not_a_whole_clip_override` - one keyframe at `3000`: `None` at `2649` and `3351`, its own pose at `3000`, and exactly `live` at both window edges (`2650`/`3350`).
- `without_a_live_pose_the_blends_snap_to_the_nearest_end_keyframe` - `live = None` still yields `None` outside the span, and the end keyframe's pose inside each blend window.
