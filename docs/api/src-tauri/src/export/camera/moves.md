# src-tauri/src/export/camera/moves.rs

Keyframed webcam PiP position+size interpolator (`EditDoc.camera_moves`), the export source of truth for the "Move" camera feature. An empty track is a no-op - the caller keeps whatever static `overlay_for` rect it already has, so a doc with no `camera_moves` composites byte-identically to today. Mirrors `LayoutTrack`'s "ease into the entered keyframe using its own easing" convention (`src-tauri/src/export/scene/layout.rs`).

## CamPose

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32 }
```

One resolved webcam-PiP position+size at a frame time.

- `x` / `y` - *the PiP's center, as a fraction (`0.0`-`1.0`) of the output frame - same units as `CameraMove.x`/`.y`.*
- `size` - *the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.*

### Used by

- `src-tauri/src/export/render/mod.rs` - `step_camera` overrides the scene's camera panel rect from a non-`None` sample (Task 4).

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
pub fn sample(&self, t_ms: u32) -> Option<CamPose>
```

Resolves the PiP pose at `t_ms`.

### Inputs

- `t_ms: u32` - video timestamp in milliseconds. *Why:* called once per output frame by the exporter, same cadence as `LayoutTrack::scene_at`.*

### Returns

- `None` if the track has no keyframes - the caller's signal to leave the scene's camera panel untouched (the "empty means default" model, same convention `LayoutTrack::scene_at`'s gap-fallback and `layout_segs_from_doc`'s `None` use).
- `Some(CamPose)` otherwise: the first keyframe's pose at/before its own `t_ms` (hold), the last keyframe's pose at/after its own `t_ms` (hold), or a component-wise lerp between the two keyframes straddling `t_ms`.

### Implementation

1. Empty `kfs` -> `None`.
2. `t_ms <= kfs[0].t_ms` -> hold the first keyframe's pose (covers `t_ms` before recording start and the exact first keyframe).
3. `t_ms >= kfs[last].t_ms` -> hold the last keyframe's pose.
4. Otherwise `t_ms` is strictly between the first and last keyframe's times: find `bi`, the first index whose `t_ms` exceeds the query, giving the straddling pair `a = kfs[bi-1]`, `b = kfs[bi]`.
5. `b.t_ms == a.t_ms` (coincident keyframe times) -> return `b`'s pose; guards the division below against a zero-length span.
6. Else `f = ease(b.easing, (t_ms - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve. *Why `b`'s easing, not `a`'s:* matches `LayoutTrack::scene_at`, which eases into a newly-entered segment using that segment's own `transition_ms`/`easing`, not the one being left.*
7. Lerp each of `x`/`y`/`size` independently: `a.field + (b.field - a.field) * f`.

### Behaviors worth knowing

- `empty_track_samples_to_none` - no keyframes -> `sample` is `None` at any `t_ms`.
- `single_keyframe_holds_at_any_time` - one keyframe -> its pose is returned for every `t_ms`, before and after.
- `holds_first_pose_before_and_at_first_keyframe` / `holds_last_pose_at_and_after_last_keyframe` - the ends hold flat instead of extrapolating.
- `midpoint_lerp_with_linear_easing_is_the_exact_mean` - `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- `midpoint_with_smooth_easing_diverges_from_the_linear_mean` - at `t=250` of a `[0,1000]` span (`u=0.25`), the sampled `x`/`size` equal `ease(Easing::Smooth, 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- `coincident_keyframe_times_snap_to_b_without_dividing_by_zero` - two keyframes sharing the same `t_ms` never panics or NaNs; sampling exactly at that time returns one of the two poses (whichever the stable sort placed at that shared time), not a lerp.
- `out_of_order_input_is_sorted_defensively` - keyframes passed to `from_doc` in descending `t_ms` order still sample correctly, proving the constructor's own sort (not caller discipline) is what keeps `sample` correct.
