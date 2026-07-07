# src/editor/stage/cameraMoves.ts

TS mirror of `CameraMoveTrack::sample` (`src-tauri/src/export/camera/moves.rs`), the export source of truth for the "Move" camera feature. Drives the webcam PiP's live-preview position+size so scrubbing the editor matches an export of the same doc frame-for-frame. An empty `camera_moves` list is a no-op - the caller keeps whatever static PiP rect it already had. A single (or first) keyframe is NOT a flat hold from `t=0`: `camMoveAt`'s optional `staticPose` acts as an implicit keyframe at `t=0`, so the PiP animates in from its static resting pose over `[0, first.t_ms]`.

## CamPose

```ts
export interface CamPose { x: number; y: number; size: number }
```

One resolved webcam-PiP position+size at a preview time.

- `x` / `y` - the PiP's center, as a fraction (`0`-`1`) of the output frame - same units as `CameraMove.x`/`.y`.
- `size` - the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.

## camMoveAt

```ts
export function camMoveAt(moves: CameraMove[], t: number, staticPose?: CamPose | null): CamPose | null
```

Resolves the PiP pose at time `t`.

### Inputs

- `moves: CameraMove[]` - `doc.camera_moves`, in any order. *Why sorted defensively:* the edit ops keep this array sorted by `t_ms`, but `camMoveAt` never trusts that from outside - it sorts a fresh copy (`[...moves].sort(...)`) rather than mutating the caller's array.
- `t: number` - the preview's current playhead time in milliseconds, same cadence as `camAt`/`layoutAt`.
- `staticPose?: CamPose | null` - the caller's un-overridden static PiP pose (e.g. derived from `PreviewLayout.cam`'s center + height), or omitted/`null`. *Why:* treated as an implicit keyframe at `t = 0` so a single (or first) real keyframe animates in from the static resting pose instead of holding flat for the whole clip; omitted/`null` reproduces the pre-existing hold-only behavior exactly, mirroring the Rust `sample`'s `static_pose: Option<CamPose>`.

### Returns

- `null` if `moves` is empty - the caller's signal to leave the PiP rect untouched (the "empty means default" model, same convention `layoutAt`'s gap-fallback uses).
- `CamPose` otherwise: at/before the first keyframe, either an ease FROM `staticPose` INTO the first keyframe (when `staticPose` is truthy and the first keyframe's `t_ms > 0`) or a flat hold of the first keyframe's pose (when `staticPose` is omitted/`null`, or the first keyframe is already at `t_ms = 0`); the last keyframe's pose at/after its own `t_ms` (hold); or a component-wise lerp between the two keyframes straddling `t`.

### Implementation

1. Sort a copy of `moves` ascending by `t_ms` - `const ks = [...moves].sort((a,b) => a.t_ms - b.t_ms)`.
2. Empty `ks` -> `null`.
3. `t <= first.t_ms` (`first = ks[0]`):
   - `staticPose` truthy AND `first.t_ms > 0` -> ease FROM `staticPose` INTO `first`: `f = ease(first.easing, t / first.t_ms)`, then `staticPose.field + (first.field - staticPose.field) * f` for each of `x`/`y`/`size`. Uses `first`'s OWN easing, the same direction step 7 below uses for `b`'s easing.
   - Otherwise -> hold `first`'s pose flat, unchanged from before this existed.
4. `t >= ks[last].t_ms` -> hold the last keyframe's pose.
5. Otherwise `t` is strictly between the first and last keyframe's times: find `bi`, the first index whose `t_ms` exceeds `t`, giving the straddling pair `a = ks[bi-1]`, `b = ks[bi]`.
6. `b.t_ms === a.t_ms` (coincident keyframe times) -> return `b`'s pose; guards the division below against a zero-length span.
7. Else `f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve, using the same `ease` (`src/editor/timeline/layoutTrack.ts`) `layoutAt` uses - no new easing implementation. *Why `b`'s easing, not `a`'s:* matches `CameraMoveTrack::sample` and `layoutAt`, which ease into a newly-entered segment using that segment's own easing, not the one being left.
8. Lerp each of `x`/`y`/`size` independently: `a.field + (b.field - a.field) * f`.

### Behaviors worth knowing

(Mirrors `moves_tests.rs` - see `docs/api/src-tauri/src/export/camera/moves.md` for the Rust-side authority these correspond to.)

- Empty track -> `null` at any `t`.
- One keyframe, `staticPose` omitted -> its pose is returned for every `t`, before and after (the pre-existing hold behavior, unchanged).
- With `staticPose` omitted, the ends hold flat instead of extrapolating.
- `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- At `t=250` of a `[0,1000]` `smooth`-eased span (`u=0.25`), the sampled `x`/`size` equal `ease("smooth", 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- Two keyframes sharing the same `t_ms` never divides by zero; sampling exactly at that time returns one of the two poses (whichever the stable sort placed at that shared time), not a lerp.
- Keyframes passed in descending `t_ms` order still sample correctly, proving `camMoveAt`'s own sort (not caller discipline) is what keeps it correct - and the caller's original array is left in its original order.
- One keyframe at `t=1000` with a `staticPose`: `camMoveAt(moves, 0, staticPose)` is (approximately) the static pose, `camMoveAt(moves, 1000, staticPose)` is the keyframe's pose, `camMoveAt(moves, 500, staticPose)` sits strictly between them on every axis (proving it animates rather than holds), and `camMoveAt(moves, 500)` (no `staticPose`) at the same `t` still returns the keyframe flat - proving the omitted-argument case reproduces the old hold behavior exactly, and matching `moves_tests.rs`'s `single_keyframe_with_static_pose_animates_in_from_the_static_start` with the same numbers.

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - overrides the webcam rect fed to `drawPreview` when non-`null` (Task 6); passes a `staticPose` derived from `baseLayout.cam`.
- `src/editor/stage/Stage.tsx` - `sampledPose` (drag-handle positioning) passes the same `staticPose` derivation so the handle matches what's drawn.

## rectFromCenter

```ts
export function rectFromCenter(p: CamPose, ow: number, oh: number): [number, number, number, number]
```

TS mirror of `rect_from_center` (`src-tauri/src/export/scene/mod.rs`) - converts a sampled `CamPose` into a fraction-of-output `[x, y, w, h]` rect (top-left form), matching the export byte-for-byte.

### Inputs

- `p: CamPose` - the sampled pose (`camMoveAt`'s result).
- `ow` / `oh: number` - the output frame's pixel dimensions. In the preview, this is the canvas's fixed 1280x720 backing store (the same basis `PreviewLayout`'s fractions use); in the export, the render's actual output size.

### Returns

`[x, y, w, h]` as fractions (`0`-`1`) of `ow`/`oh` respectively - the same shape as the first four elements of `PreviewLayout.cam`.

### Implementation

1. `h = p.size * oh` - the square's side length in PIXELS, derived from height only (matches the Rust: width is copied from the height-derived side, not computed independently).
2. Return `[(p.x*ow - h/2)/ow, (p.y*oh - h/2)/oh, h/ow, h/oh]` - the pixel-space top-left rect (`x*ow - h/2`, `y*oh - h/2`, `h`, `h`), each component then divided back into a fraction by its own axis (`ow` for x/w, `oh` for y/h).

### Notes

- **Why divide by `ow` and `oh` separately, not a single scalar:** the rect is a true PIXEL square (both sides equal `h`), but `ow != oh` for a 16:9 output, so the fraction-space `w` (`h/ow`) and `h` (`h/oh`) come out different from each other - exactly mirroring what `preview_layout`'s Rust command does when it converts the export's pixel rect to fractions (`r.w / ow, r.h / oh`). The caller (`useCompositeLoop.ts`) multiplies these fractions back by the canvas's own `ow`/`oh` before drawing, so the pixel math round-trips exactly and the square renders as a true square, matching the export.
- Radius is not part of this helper's output - the caller splices the existing (unoverridden) radius back in, scaled via `radiusScaleForResize` (Task 9 Part C) so the override composes correctly with a resize; "leave the camera panel radius as resolved AND SCALE IT" is the caller's contract, not this formula's.

## radiusScaleForResize

```ts
export function radiusScaleForResize(oldH: number, newH: number): number
```

Task 9 Part C. TS mirror of `override_camera`'s radius-scaling factor (`export/scene/mod.rs`) - the fix for a pre-existing bug where a camera_moves keyframe replaced only the panel's rect, leaving its radius fraction at the STATIC value, so a circle panel distorted (stayed circle-shaped in mask, but no longer matched a true `min(w,h)/2` circle) whenever a keyframe grew or shrank it. Multiplying the existing radius fraction by this factor keeps a circle a true circle after the resize.

### Inputs

- `oldH: number` - the panel's height BEFORE the override (`PreviewLayout.cam[3]`, a fraction of `oh`).
- `newH: number` - the panel's height AFTER the override (`rectFromCenter(...)[3]`, same fraction-of-`oh` units).

### Returns

`newH / Math.max(oldH, 0.001)` - a dimensionless ratio (units cancel, so it is valid to apply to a WIDTH-relative fraction like `PreviewLayout.cam[4]` even though `oldH`/`newH` are themselves HEIGHT-relative - see `previewCanvas.ts`'s `wr = fr * w`, confirming `cam[4]` is always `radius_px / ow`). Guards `oldH` against 0/near-0 the same way the Rust does (`old_h.max(0.001)`), so a degenerate zero-height static panel never divides by zero.

### Behaviors worth knowing

- `radiusScaleForResize(0.2, 0.1)` returns `0.5` (shrink to half height -> half radius scale).
- `radiusScaleForResize(0.1, 0.4)` returns `4.0` (grow to 4x height -> 4x radius scale).
- `radiusScaleForResize(0.25, 0.25)` returns `1` (no size change -> no-op scale).
- An end-to-end test converts a circle's radius fraction to PIXELS on both axes (respecting `cam[4]`'s WIDTH-relative units vs. `cam[3]`'s HEIGHT-relative units) and confirms `scaledRadiusPx == newHPx / 2` - still a true circle - after resize.

### Used by

- `overrideCamPanel` (below) - scales both the spliced radius and ring width by this same factor.

## overrideCamPanel

```ts
export function overrideCamPanel(
  baseCam: [number, number, number, number, number, number, number, number, number],
  p: CamPose, ow: number, oh: number,
): [number, number, number, number, number, number, number, number, number]
```

TS mirror of `override_camera` (`export/scene/mod.rs`) end-to-end: combines `rectFromCenter` + `radiusScaleForResize` into the single call site `useCompositeLoop.ts` needs, so that file doesn't have to re-derive the splice inline.

### Inputs

- `baseCam` - the resolved (unoverridden) `PreviewLayout.cam` tuple: `[x, y, w, h, radius, ringPx, ringR, ringG, ringB]`.
- `p: CamPose`, `ow`/`oh: number` - same as `rectFromCenter`.

### Returns

A new 9-tuple: `rectFromCenter(p, ow, oh)`'s `[x,y,w,h]`, followed by `baseCam[4]` (radius) and `baseCam[5]` (ring width) each scaled by `radiusScaleForResize(baseCam[3], newRect[3])`, followed by `baseCam[6..9]` (ring color) unchanged. Mirrors the Rust `Panel { rect, radius: radius*m, ring_px: ring_px*m, ..panel }` - alpha and ring color are never touched by the override, only rect/radius/ring-width scale.

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - replaces `baseLayout.cam` with `overrideCamPanel(baseLayout.cam, cp, c.width, c.height)` when a camera_moves/drag pose is active and a camera panel is resolved this frame, so the preview's PiP rect, radius, AND ring stay in parity with the export's `override_camera` after a keyframe resize.
