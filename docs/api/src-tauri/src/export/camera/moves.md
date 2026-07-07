# src-tauri/src/export/camera/moves.rs

Keyframed webcam PiP position+size interpolator (`EditDoc.camera_moves`), the export source of truth for the "Move" camera feature. An empty track is a no-op - the caller keeps whatever static `overlay_for` rect it already has, so a doc with no `camera_moves` composites byte-identically to today. Mirrors `LayoutTrack`'s "ease into the entered keyframe using its own easing" convention (`src-tauri/src/export/scene/layout.rs`). A single (or first) keyframe is NOT a flat hold from `t=0`: `sample`'s caller-supplied `static_pose` acts as an implicit keyframe at `t=0`, so the PiP animates in from its static resting pose over `[0, first.t_ms]` - see `CameraMoveTrack::sample`.

## CamPose

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32 }
```

One resolved webcam-PiP position+size at a frame time.

- `x` / `y` - *the PiP's center, as a fraction (`0.0`-`1.0`) of the output frame - same units as `CameraMove.x`/`.y`.*
- `size` - *the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.*

### Used by

- `src-tauri/src/export/render/mod.rs` - `step_camera` overrides the scene's camera panel rect from a non-`None` sample (Task 4); also the shape `static_cam_pose` (`export/camera/mod.rs`) returns, passed as `sample`'s implicit-start-keyframe pose.

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

## CameraMoveTrack::sample

```rust
pub fn sample(&self, t_ms: u32, static_pose: Option<CamPose>) -> Option<CamPose>
```

Resolves the PiP pose at `t_ms`.

### Inputs

- `t_ms: u32` - video timestamp in milliseconds. *Why:* called once per output frame by the exporter, same cadence as `LayoutTrack::scene_at`.*
- `static_pose: Option<CamPose>` - the caller's un-overridden static PiP pose (e.g. `export::camera::static_cam_pose` of the resolved-but-not-yet-overridden `scene.camera.rect`), or `None`. *Why:* treated as an implicit keyframe at `t_ms = 0` so a single (or first) real keyframe animates in from the static resting pose instead of holding flat for the whole clip; `None` reproduces the pre-existing hold-only behavior exactly (every call site that hasn't been updated for this, and the "no static pose available" case, keep working unchanged).

### Returns

- `None` if the track has no keyframes - the caller's signal to leave the scene's camera panel untouched (the "empty means default" model, same convention `LayoutTrack::scene_at`'s gap-fallback and `layout_segs_from_doc`'s `None` use).
- `Some(CamPose)` otherwise: at/before the first keyframe, either an ease FROM `static_pose` INTO the first keyframe (when `static_pose` is `Some` and the first keyframe's `t_ms > 0`) or a flat hold of the first keyframe's pose (when `static_pose` is `None`, or the first keyframe is already at `t_ms = 0`, i.e. nothing to ease from); the last keyframe's pose at/after its own `t_ms` (hold); or a component-wise lerp between the two keyframes straddling `t_ms`.

### Implementation

1. Empty `kfs` -> `None`.
2. `t_ms <= first.t_ms` (`first = kfs[0]`):
   - `static_pose = Some(s)` AND `first.t_ms > 0` -> ease FROM `s` INTO `first`: `f = ease(first.easing, t_ms / first.t_ms)`, then `s.field + (first.field - s.field) * f` for each of `x`/`y`/`size`. Uses `first`'s OWN easing, the same "ease into the entered segment" direction step 6 below uses for `b`'s easing - not a new convention.
   - Otherwise (`static_pose = None`, or `first.t_ms == 0`) -> hold `first`'s pose flat, unchanged from before this existed.
3. `t_ms >= kfs[last].t_ms` -> hold the last keyframe's pose.
4. Otherwise `t_ms` is strictly between the first and last keyframe's times: find `bi`, the first index whose `t_ms` exceeds the query, giving the straddling pair `a = kfs[bi-1]`, `b = kfs[bi]`.
5. `b.t_ms == a.t_ms` (coincident keyframe times) -> return `b`'s pose; guards the division below against a zero-length span.
6. Else `f = ease(b.easing, (t_ms - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve. *Why `b`'s easing, not `a`'s:* matches `LayoutTrack::scene_at`, which eases into a newly-entered segment using that segment's own `transition_ms`/`easing`, not the one being left.*
7. Lerp each of `x`/`y`/`size` independently: `a.field + (b.field - a.field) * f`.

### Behaviors worth knowing

- `empty_track_samples_to_none` - no keyframes -> `sample` is `None` at any `t_ms`.
- `single_keyframe_holds_at_any_time` - one keyframe, `static_pose = None` -> its pose is returned for every `t_ms`, before and after (the pre-existing hold behavior, unchanged).
- `holds_first_pose_before_and_at_first_keyframe` / `holds_last_pose_at_and_after_last_keyframe` - with `static_pose = None`, the ends hold flat instead of extrapolating.
- `midpoint_lerp_with_linear_easing_is_the_exact_mean` - `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- `midpoint_with_smooth_easing_diverges_from_the_linear_mean` - at `t=250` of a `[0,1000]` span (`u=0.25`), the sampled `x`/`size` equal `ease(Easing::Smooth, 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- `coincident_keyframe_times_snap_to_b_without_dividing_by_zero` - two keyframes sharing the same `t_ms` never panics or NaNs; sampling exactly at that time returns one of the two poses (whichever the stable sort placed at that shared time), not a lerp.
- `out_of_order_input_is_sorted_defensively` - keyframes passed to `from_doc` in descending `t_ms` order still sample correctly, proving the constructor's own sort (not caller discipline) is what keeps `sample` correct.
- `single_keyframe_with_static_pose_animates_in_from_the_static_start` - one keyframe at `t=1000` with `static_pose = Some(...)`: `sample(0, static)` is (approximately) the static pose, `sample(1000, static)` is the keyframe's pose, `sample(500, static)` sits strictly between them on every axis (proving it animates rather than holds), and `sample(500, None)` at the same `t_ms` still returns the keyframe flat - proving `None` reproduces the old hold behavior exactly.
