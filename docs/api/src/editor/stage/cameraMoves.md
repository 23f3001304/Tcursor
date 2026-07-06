# src/editor/stage/cameraMoves.ts

TS mirror of `CameraMoveTrack::sample` (`src-tauri/src/export/camera/moves.rs`), the export source of truth for the "Move" camera feature. Drives the webcam PiP's live-preview position+size so scrubbing the editor matches an export of the same doc frame-for-frame. An empty `camera_moves` list is a no-op - the caller keeps whatever static PiP rect it already had.

## CamPose

```ts
export interface CamPose { x: number; y: number; size: number }
```

One resolved webcam-PiP position+size at a preview time.

- `x` / `y` - the PiP's center, as a fraction (`0`-`1`) of the output frame - same units as `CameraMove.x`/`.y`.
- `size` - the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.

## camMoveAt

```ts
export function camMoveAt(moves: CameraMove[], t: number): CamPose | null
```

Resolves the PiP pose at time `t`.

### Inputs

- `moves: CameraMove[]` - `doc.camera_moves`, in any order. *Why sorted defensively:* the edit ops keep this array sorted by `t_ms`, but `camMoveAt` never trusts that from outside - it sorts a fresh copy (`[...moves].sort(...)`) rather than mutating the caller's array.
- `t: number` - the preview's current playhead time in milliseconds, same cadence as `camAt`/`layoutAt`.

### Returns

- `null` if `moves` is empty - the caller's signal to leave the PiP rect untouched (the "empty means default" model, same convention `layoutAt`'s gap-fallback uses).
- `CamPose` otherwise: the first keyframe's pose at/before its own `t_ms` (hold), the last keyframe's pose at/after its own `t_ms` (hold), or a component-wise lerp between the two keyframes straddling `t`.

### Implementation

1. Sort a copy of `moves` ascending by `t_ms` - `const ks = [...moves].sort((a,b) => a.t_ms - b.t_ms)`.
2. Empty `ks` -> `null`.
3. `t <= ks[0].t_ms` -> hold the first keyframe's pose.
4. `t >= ks[last].t_ms` -> hold the last keyframe's pose.
5. Otherwise `t` is strictly between the first and last keyframe's times: find `bi`, the first index whose `t_ms` exceeds `t`, giving the straddling pair `a = ks[bi-1]`, `b = ks[bi]`.
6. `b.t_ms === a.t_ms` (coincident keyframe times) -> return `b`'s pose; guards the division below against a zero-length span.
7. Else `f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve, using the same `ease` (`src/editor/timeline/layoutTrack.ts`) `layoutAt` uses - no new easing implementation. *Why `b`'s easing, not `a`'s:* matches `CameraMoveTrack::sample` and `layoutAt`, which ease into a newly-entered segment using that segment's own easing, not the one being left.
8. Lerp each of `x`/`y`/`size` independently: `a.field + (b.field - a.field) * f`.

### Behaviors worth knowing

(Mirrors `moves_tests.rs` - see `docs/api/src-tauri/src/export/camera/moves.md` for the Rust-side authority these correspond to.)

- Empty track -> `null` at any `t`.
- One keyframe -> its pose is returned for every `t`, before and after.
- The ends hold flat instead of extrapolating.
- `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- At `t=250` of a `[0,1000]` `smooth`-eased span (`u=0.25`), the sampled `x`/`size` equal `ease("smooth", 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- Two keyframes sharing the same `t_ms` never divides by zero; sampling exactly at that time returns one of the two poses (whichever the stable sort placed at that shared time), not a lerp.
- Keyframes passed in descending `t_ms` order still sample correctly, proving `camMoveAt`'s own sort (not caller discipline) is what keeps it correct - and the caller's original array is left in its original order.

### Used by

- `src/editor/stage/useCompositeLoop.ts` - overrides the webcam rect fed to `drawPreview` when non-`null` (Task 6).
