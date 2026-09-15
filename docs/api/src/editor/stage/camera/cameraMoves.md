# src/editor/stage/camera/cameraMoves.ts

TS mirror of `CameraMoveTrack::sample` (`src-tauri/src/export/camera/moves.rs`), the export source of truth for the "Move" camera feature. Drives the webcam PiP's live-preview position, size and shape so scrubbing the editor matches an export of the same doc frame-for-frame. An empty `camera_moves` list is a no-op - the caller keeps whatever layout-resolved PiP rect it already had.

**Task 27 - keyframes own only their span.** The keyframes govern exactly `[first - KF_BLEND_MS, last + KF_BLEND_MS]`; outside it `camMoveAt` is `null` and the layout segments own the panel, as if no keyframes existed. The old `staticPose` argument (an implicit `t=0` keyframe, which made one keyframe override the WHOLE clip) became `live`: the layout-resolved pose for THIS frame, which the track eases out of entering the span and back into leaving it - re-read every frame, so an exit blend chases a layout cross-fade that is still moving. Same five cases, same `KF_BLEND_MS`, as the Rust.

**Shapes (2026-09-14).** A keyframe carries its own shape (`CameraMove.shape`/`roundness`), folded into the pose as `round` - a corner fraction of the panel's short side - and lerped between keyframes exactly like the rect, so a bubble can morph from a circle into a rounded card as it moves. `"layout"` (every keyframe written before this) inherits the live panel's shape, so old docs render as they always did.

## CamPose

```ts
export interface CamPose { x: number; y: number; size: number; round?: number }
```

One resolved webcam-PiP pose at a preview time.

- `x` / `y` - the PiP's center, as a fraction (`0`-`1`) of the output frame - same units as `CameraMove.x`/`.y`.
- `size` - the PiP's height, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the width from the panel's aspect.
- `round` - the corner radius as a fraction of the panel's SHORT side (`0` = rect, `0.5` = circle). Absent means "the static panel's radius, scaled with the resize": a drag draft, an arrangement pose, or a layout-shaped keyframe sampled with no live pose to inherit from. Rust `CamPose`'s `round: Option<f32>`.

## KF_BLEND_MS

```ts
export const KF_BLEND_MS = 350;
```

Handoff length in milliseconds on EACH side of the keyframe span, between the live layout-resolved pose and the track. Mirrors the Rust `KF_BLEND_MS` (`src-tauri/src/export/camera/moves.rs`) and MUST stay numerically equal to it - a module constant, deliberately not a setting. Also read by `src/editor/timeline/lanes/CameraLane.tsx` to draw the span bar's two fade ramps at exactly the width the sampler actually blends over.

## camKfRange

```ts
export function camKfRange(moves: CameraMove[]): [number, number] | null
```

The RAW `[first, last]` keyframe times of a track (unsorted input is fine - it scans for min/max rather than sorting), or `null` when the track is empty.

**This is deliberately NOT the ownership window, and not the counterpart of Rust's `CameraMoveTrack::span`.** `span()` returns the window the track OWNS - this range padded by `KF_BLEND_MS` on each side and clamped at `0` - which is what `camMoveAt` tests `t` against. `camKfRange` returns the unpadded ends, because both of its consumers want exactly that and each does something different with it. The name says `KfRange`, not `Span`, precisely so the two are not mistaken for mirrors of each other.

### Used by

- `src/editor/panels/camera/CameraMoveField.tsx` - clamps the playhead **into** this range (`min(max(t, first), last)`) to seed a keyframe ADDED outside the ownership window from the nearest end of the track, rather than jumping to frame-centre now that `camMoveAt` is `null` out there. Clamping into the padded window would land inside a blend and sample a partly-blended pose, so the unpadded range is the correct input here.
- `src/editor/timeline/lanes/CameraLane.tsx` does **not** call this: its span bar must follow a diamond mid-drag, so it pads its own drag-adjusted keyframe list (`kfs[0].t - KF_BLEND_MS` … `kfs[at end].t + KF_BLEND_MS`) inline instead of reading committed doc values.

## shapeRound

```ts
export function shapeRound(shape: string, roundness: number): number | null
```

A keyframe's `shape`/`roundness` as the corner fraction `CamPose.round` carries: `"circle"` -> `0.5`, `"rect"` -> `0`, `"rounded"` -> `roundness` clamped to `[0, 0.5]`, anything else (`"layout"`) -> `null` = inherit. Mirrors Rust `shape_round`.

## camMoveAt

```ts
export function camMoveAt(moves: CameraMove[], t: number, live?: CamPose | null): CamPose | null
```

Resolves the PiP pose at time `t`, or `null` when the keyframes do not own that frame.

### Inputs

- `moves: CameraMove[]` - `doc.camera_moves`, in any order. *Why sorted defensively:* the edit ops keep this array sorted by `t_ms`, but `camMoveAt` never trusts that from outside - it sorts a fresh copy (`[...moves].sort(...)`) rather than mutating the caller's array.
- `t: number` - the preview's current playhead time in milliseconds (output clock), same cadence as `camAt`/`layoutAt`.
- `live?: CamPose | null` - the LIVE layout-resolved PiP pose for THIS frame (`liveCamPose` of `PreviewLayout.cam`, i.e. what `layoutAt` resolved before any override), or omitted/`null`. *Why per-frame, not a one-off static pose:* it is what the two blends ease to and from, and a layout cross-fade moves it while the exit blend is running; it is also the shape a `"layout"` keyframe inherits. Omitted/`null` skips both blends (each snaps to the nearest end keyframe) and leaves an inheriting keyframe's `round` undefined; the span rule itself never depends on it. Mirrors the Rust `sample`'s `live: Option<CamPose>`.

### Returns

- `null` if `moves` is empty, or `t` is outside `[first - KF_BLEND_MS, last + KF_BLEND_MS]` - the caller's signal to leave the PiP rect exactly as the layout resolved it (the "empty means default" model, same convention `layoutAt`'s gap-fallback uses).
- `CamPose` inside that span, per the five cases below.

### Implementation

1. Sort a copy of `moves` ascending by `t_ms` - `const ks = [...moves].sort((a,b) => a.t_ms - b.t_ms)`; empty -> `null`.
2. `entry = max(0, first.t_ms - KF_BLEND_MS)`; `t < entry || t > last.t_ms + KF_BLEND_MS` -> `null`. **Case 1.**
3. `pose(k)` is a keyframe as a `CamPose`: its x/y/size, and `round = shapeRound(k.shape, k.roundness) ?? live?.round` - its own shape, else the live panel's, else undefined.
4. `t < first.t_ms` - **case 2, the entry blend:** `mix(live, pose(first), ease(first.easing, (t - entry) / (first.t_ms - entry)))`. Continuous at both ends (exactly `live` at `entry`, exactly the keyframe pose at `first`). No `live`, or a zero-length window (a keyframe at `t_ms = 0`) -> the first keyframe's pose.
5. `t > last.t_ms` - **case 4, the exit blend:** `mix(pose(last), live, ease(last.easing, (t - last.t_ms) / KF_BLEND_MS))`. No `live` -> the last keyframe's pose.
6. `t >= last.t_ms` -> the last keyframe's pose (also the single-keyframe instant, **case 5**: a lone keyframe eases in, hits its pose, eases back out - a hold needs two keyframes).
7. **Case 3, in-span - unchanged math:** find `bi`, the first index whose `t_ms` exceeds `t`, giving the straddling pair `a = ks[bi-1]`, `b = ks[bi]`. `b.t_ms === a.t_ms` (coincident keyframe times) -> `b`'s pose, guarding the division. Else `f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve, using the same `ease` (`src/editor/timeline/model/layoutTrack.ts`) `layoutAt` uses - no new easing implementation. *Why `b`'s easing, not `a`'s:* matches `CameraMoveTrack::sample` and `layoutAt`, which ease into a newly-entered segment using that segment's own easing, not the one being left.
8. Every blend and interpolation goes through the module-private `mix`, a component-wise lerp of `x`/`y`/`size`/`round` (a `round` only one side knows is carried through unblended; two unknowns stay unknown); `rectFromCenter` derives the rect from the RESULT once, so the panel's aspect handling applies to a blended pose too.

### Behaviors worth knowing

(Mirrors `moves_tests.rs` / `moves_span_tests.rs` - see `docs/api/src-tauri/src/export/camera/moves.md` for the Rust-side authority these correspond to. TS-side they are split the same way: `cameraMoves.test.ts` for the sampling, the resize math and the key, `cameraMovesShape.test.ts` for the keyframe shapes and the panel override.)

- Empty track -> `null` at any `t`.
- `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- At `t=250` of a `[0,1000]` `smooth`-eased span (`u=0.25`), the sampled `x`/`size` equal `ease("smooth", 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- Two keyframes sharing the same `t_ms` never divides by zero; sampling exactly at that time returns one of the two poses, not a lerp.
- Keyframes passed in descending `t_ms` order still sample correctly, proving `camMoveAt`'s own sort (not caller discipline) is what keeps it correct - and the caller's original array is left in its original order.
- Keyframes at `2000`/`4000` sampled at `3000` reproduce the pre-Task-27 value (`x = 0.40`, pinned literally) and are identical with and without a `live` pose - `live` has no influence mid-span. Same numbers as the Rust `mid_span_interpolation_is_unchanged_by_the_span_rewrite`.
- With keyframes at `2000`/`4000` (span `[1650, 4350]`): `1000`/`5000` are `null`; `1649`/`4351` are `null` while `1650`/`4350` are not (the edges are inside); the entry blend equals `live` at `1650` and the keyframe pose at `2000` within `1e-4`, with the midpoint strictly between; and with a `live` pose sweeping linearly across the exit window, the blend at `4350` lands on `live(4350)` within `1e-4`, not the stale `live(4000)`.
- One keyframe at `3000` is a bump: `null` at `2649` and `3351`, its own pose at `3000`, and exactly `live` at both window edges.
- A circle keyframe at `0` and a rect keyframe at `1000` sample `round` `0.5`, `0`, and `0.25` at `500` (linear); a `"rounded"` keyframe carries its own roundness; `shapeRound("rounded", 9)` clamps to `0.5`.
- A `"layout"` keyframe samples the live pose's `round` (`0.1`) and `undefined` without one; half way to a circle keyframe the inherited `0.1` morphs to `0.3`; with no live pose the circle's `0.5` is carried rather than invented.
- `KF_BLEND_MS` is asserted to be `350` in the TS test as well, so a change on one side without the other fails immediately.

### Used by

- `src/editor/stage/camera/frameCam.ts` - `frameCamLayout` overrides the webcam rect fed to `drawPreview` when non-`null`, and falls through to the smart webcam-on-zoom action when `null` (which is now also the case outside the span); passes `liveCamPose(base.cam, ...)` as `live`.
- `src/editor/stage/camera/CamDragHandle.tsx` - `sampledPose` (drag-handle positioning) passes the same live pose so the handle matches what's drawn, and falls back to the layout's panel when the sample is `null`.
- `src/editor/panels/camera/CameraMoveField.tsx` - the Move-mode "Webcam size" slider reads the keyframed size at the playhead, falling back to the static `cam_size` wherever the sample is `null`.

## rectFromCenter

```ts
export function rectFromCenter(p: CamPose, ow: number, oh: number, aspect: number): [number, number, number, number]
```

TS mirror of `rect_from_center` (`src-tauri/src/export/scene/mod.rs`) - converts a sampled `CamPose` into a fraction-of-output `[x, y, w, h]` rect (top-left form), matching the export byte-for-byte.

### Inputs

- `p: CamPose` - the sampled pose (`camMoveAt`'s result); `round` is not read here.
- `ow` / `oh: number` - the output frame's pixel dimensions. In the preview, this is the canvas's backing store, sized from `PreviewLayout.canvas` (the same basis `PreviewLayout`'s fractions use, following the doc's chosen aspect); in the export, the render's actual output size.
- `aspect: number` - the PiP panel's own width/height IN PIXELS (`1` square, `16/9` for a Wide panel), from `camAspect(baseLayout.cam, ow, oh)`. *Why the caller supplies it:* a `CamPose` carries height only, so without an aspect a single keyframe squared a Wide panel for the whole clip. Clamped to `>= 0.01`, matching the Rust `aspect.max(0.01)`.

### Returns

`[x, y, w, h]` as fractions (`0`-`1`) of `ow`/`oh` respectively - the same shape as the first four elements of `PreviewLayout.cam`.

### Implementation

1. `h = p.size * oh` - the panel height in PIXELS, from the pose's `size` only (the pose never carries width).
2. `w = h * max(aspect, 0.01)` - the width in PIXELS, restored from the STATIC panel's shape (matches the Rust `w = h * aspect.max(0.01)`).
3. Return `[(p.x*ow - w/2)/ow, (p.y*oh - h/2)/oh, w/ow, h/oh]` - the pixel-space top-left rect, each component then divided back into a fraction by its own axis (`ow` for x/w, `oh` for y/h).

### Notes

- **Why divide by `ow` and `oh` separately, not a single scalar:** the rect's shape is defined in PIXELS (`w = h * aspect`), but `ow != oh` for a 16:9 output, so the fraction-space `w` (`w/ow`) and `h` (`h/oh`) come out different from each other - exactly mirroring what `preview_layout`'s Rust command does when it converts the export's pixel rect to fractions (`r.w / ow, r.h / oh`). The caller (`useCompositeLoop.ts`) multiplies these fractions back by the canvas's own `ow`/`oh` before drawing, so the pixel math round-trips exactly and the square renders as a true square, matching the export.
- Radius is not part of this helper's output - `overrideCamPanel` sets it, from the pose's own `round` or by scaling the existing one via `radiusScaleForResize`; "leave the camera panel radius as resolved AND SCALE IT" is the caller's contract, not this formula's.

## camAspect

```ts
export function camAspect(cam: [number, number, number, number, ...number[]], ow: number, oh: number): number
```

The static PiP panel's PIXEL aspect (w/h) read off a `PreviewLayout.cam` tuple - the `aspect` `rectFromCenter` needs.

### Inputs

- `cam` - a `PreviewLayout.cam` tuple (`[x, y, w, h, radius, ringPx, r, g, b]`); only `w`/`h` are read.
- `ow` / `oh: number` - the output frame's pixel dims.

### Returns

`(cam[2] * ow) / max(cam[3] * oh, 0.001)`. *Why the conversion back to pixels:* `cam[2]` and `cam[3]` are fractions of DIFFERENT axes (`ow` and `oh`), so their raw ratio is not the panel's aspect on a non-square frame. The `0.001` floor mirrors the Rust `rect.h.max(0.001)`.

### Notes

- Mirrors `FrameRenderer::step_camera` reading `scene.camera.rect.w / scene.camera.rect.h` (export/render/mod.rs) rather than re-deriving the aspect from `appearance.cam_aspect`: the resolved panel is the truth for whichever preset (or cross-fade between presets) is live this frame.

### Used by

- `overrideCamPanel` (below), so the handle, the drawn preview, and the export all agree on a Wide panel's shape.

## liveCamPose

```ts
export function liveCamPose(cam: [number, number, number, number, number, ...number[]], ow: number, oh: number): CamPose
```

TS mirror of `static_cam_pose` (`src-tauri/src/export/camera/mod.rs`): the layout-resolved PiP panel as the pose the keyframe track eases out of and back into, and whose `round` a `"layout"`-shaped keyframe inherits.

### Inputs

- `cam` - `PreviewLayout.cam` (`[x, y, w, h, r, ...]`; `r` is the radius as a fraction of the output WIDTH, like x/w - `previewCanvas.ts` draws it as `fr * w`).
- `ow` / `oh: number` - the output frame's pixel dims.

### Returns

`{ x: cam[0] + cam[2]/2, y: cam[1] + cam[3]/2, size: cam[3], round: (cam[4] * ow) / max(min(cam[2]*ow, cam[3]*oh), 0.001) }` - centre, height fraction, and the radius over the panel's short side in px (the `0.001` floor mirrors the Rust `.max(0.001)`).

### Used by

- `src/editor/stage/camera/frameCam.ts` and `src/editor/stage/camera/CamDragHandle.tsx` - the one derivation of the live pose, so the composite and the handle cannot disagree about what a keyframe inherits.

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

- `overrideCamPanel` (below) - scales the ring width, and the radius of a pose without a shape of its own, by this factor.

## overrideCamPanel

```ts
export function overrideCamPanel(
  baseCam: [number, number, number, number, number, number, number, number, number],
  p: CamPose, ow: number, oh: number,
): [number, number, number, number, number, number, number, number, number]
```

TS mirror of `override_camera` (`export/scene/mod.rs`) end-to-end: combines `rectFromCenter` + the radius rule + `radiusScaleForResize` into the single call site `useCompositeLoop.ts`/`CamDragHandle.tsx` need, so neither re-derives the splice inline.

### Inputs

- `baseCam` - the resolved (unoverridden) `PreviewLayout.cam` tuple: `[x, y, w, h, radius, ringPx, ringR, ringG, ringB]`.
- `p: CamPose`, `ow`/`oh: number` - same as `rectFromCenter`. The `aspect` is NOT a parameter here: it is derived from `baseCam` itself via `camAspect`, so the override can never disagree with the panel it is overriding (mirroring the renderer reading it off `scene.camera.rect`).

### Returns

A new 9-tuple: `rectFromCenter(p, ow, oh, camAspect(baseCam, ow, oh))`'s `[x,y,w,h]`, then the radius - `p.round * min(w_px, h_px) / ow` when the pose carries a shape (a keyframed circle, rounded rect or rect, and the morph between them - the fraction of the NEW short side, handed back in the tuple's own fraction-of-`ow` unit), else `baseCam[4]` scaled by `radiusScaleForResize(baseCam[3], newRect[3])` - then `baseCam[5]` (ring width) scaled by that same factor, followed by `baseCam[6..9]` (ring color) unchanged. Mirrors the Rust `Panel { rect, radius, ring_px: ring_px*m, ..panel }` - alpha and ring color are never touched by the override.

### Behaviors worth knowing

- A 100x100 px static panel with a 12 px radius on a 1000x1000 canvas, posed at `size 0.4`: `round 0.5` gives a radius fraction `0.2` (200 px, half the 400 px side); `round 0` gives `0`; no `round` gives `0.048` (the static 12 px scaled 4x). `liveCamPose` of the same panel reads `round 0.12`.

### Used by

- `src/editor/stage/camera/frameCam.ts` - replaces `baseLayout.cam` with `overrideCamPanel(baseLayout.cam, cp, c.width, c.height)` when a camera_moves/drag pose is active and a camera panel is resolved this frame, so the preview's PiP rect, radius, AND ring stay in parity with the export's `override_camera`.
- `src/editor/stage/camera/CamDragHandle.tsx` - the same call, so the handle's box and rounding hug the panel the composite is drawing, keyframed shape included.

## cameraMovesKey

```ts
export function cameraMovesKey(moves: CameraMove[]): string
```

A cheap CONTENT signature for `moves` - `id:t_ms:x:y:size:easing:shape:roundness` per entry, joined with `|` in array order. Added bug-sweep-2 Task 8 review round 2 (Important).

### Why this exists

`applyEditOp` round-trips the WHOLE `EditDoc` through IPC, so every call - add a zoom, delete a region, trim, an AI-director step, not just a camera-move edit - hands `setDoc` a brand-new `EditDoc`, and therefore a brand-new `camera_moves` ARRAY REFERENCE, even when its content is byte-identical to before. A consumer that needs to react to camera_moves actually CHANGING (not merely "the doc changed, for any reason") cannot key a `useEffect` on `[cameraMoves]` directly - that fires on every single edit. `cameraMovesKey` gives such a consumer something to diff instead: compute it before and after, and treat a differing key as "camera_moves actually changed."

### Returns

A string built from every field two `CameraMove`s could differ in (`id`, `t_ms`, `x`, `y`, `size`, `easing`, `shape`, `roundness`) - two calls return the same string iff every entry, in the same array order, has identical values in all eight fields. Order-sensitive: a same-set reorder still counts as a change (deliberate - see `cameraMoves.test.ts`; in practice this is fine because `add_camera_move`'s server-side sort makes order a function of `t_ms`, so a genuine reorder only happens alongside a real content change anyway). Deterministic and pure - no hashing, just string concatenation, cheap enough to call on every render of a small `camera_moves` array.

### Used by

`src/editor/stage/camera/CamDragHandle.tsx` - gates its Move-mode-draft mirror-clear effect (`useEffect(() => {...}, [cameraMoves])`) on this instead of the raw reference: stores the key in a ref, recomputes it whenever the effect fires (which is still every edit, since the effect itself is still keyed on the reference), and only calls `setDragPose(null)` when the key actually differs from what it stored last. Fixes the exact repro the review caught: Move-mode drag the PiP (an uncommitted draft), press Z to add a zoom - the OLD reference-keyed effect nulled the local `dragPose` mirror on that unrelated edit, snapping the drag-handle overlay to `sampledPose` while the composited canvas (still reading the un-cleared `camDraftRef.current`) kept drawing the actual drag position, two on-screen elements visibly disagreeing. See `CamDragHandle.md`.
