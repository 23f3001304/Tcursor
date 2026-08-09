# src/editor/stage/cameraMoves.ts

TS mirror of `CameraMoveTrack::sample` (`src-tauri/src/export/camera/moves.rs`), the export source of truth for the "Move" camera feature. Drives the webcam PiP's live-preview position+size so scrubbing the editor matches an export of the same doc frame-for-frame. An empty `camera_moves` list is a no-op - the caller keeps whatever layout-resolved PiP rect it already had.

**Task 27 - keyframes own only their span.** The keyframes govern exactly `[first - KF_BLEND_MS, last + KF_BLEND_MS]`; outside it `camMoveAt` is `null` and the layout segments own the panel, as if no keyframes existed. The old `staticPose` argument (an implicit `t=0` keyframe, which made one keyframe override the WHOLE clip) became `live`: the layout-resolved pose for THIS frame, which the track eases out of entering the span and back into leaving it - re-read every frame, so an exit blend chases a layout cross-fade that is still moving. Same five cases, same `KF_BLEND_MS`, as the Rust.

## CamPose

```ts
export interface CamPose { x: number; y: number; size: number }
```

One resolved webcam-PiP position+size at a preview time.

- `x` / `y` - the PiP's center, as a fraction (`0`-`1`) of the output frame - same units as `CameraMove.x`/`.y`.
- `size` - the PiP's size, as a fraction of the output frame - same units as `CameraMove.size`. The caller derives the other dimension from the mode's aspect.

## KF_BLEND_MS

```ts
export const KF_BLEND_MS = 350;
```

Handoff length in milliseconds on EACH side of the keyframe span, between the live layout-resolved pose and the track. Mirrors the Rust `KF_BLEND_MS` (`src-tauri/src/export/camera/moves.rs`) and MUST stay numerically equal to it - a module constant, deliberately not a setting. Also read by `src/editor/timeline/CameraLane.tsx` to draw the span bar's two fade ramps at exactly the width the sampler actually blends over.

## camKfRange

```ts
export function camKfRange(moves: CameraMove[]): [number, number] | null
```

The RAW `[first, last]` keyframe times of a track (unsorted input is fine - it scans for min/max rather than sorting), or `null` when the track is empty.

**This is deliberately NOT the ownership window, and not the counterpart of Rust's `CameraMoveTrack::span`.** `span()` returns the window the track OWNS - this range padded by `KF_BLEND_MS` on each side and clamped at `0` - which is what `camMoveAt` tests `t` against. `camKfRange` returns the unpadded ends, because both of its consumers want exactly that and each does something different with it. The name says `KfRange`, not `Span`, precisely so the two are not mistaken for mirrors of each other.

### Used by

- `src/editor/panels/CameraPanel.tsx` - clamps the playhead **into** this range (`min(max(t, first), last)`) to seed a keyframe ADDED outside the ownership window from the nearest end of the track, rather than jumping to frame-centre now that `camMoveAt` is `null` out there. Clamping into the padded window would land inside a blend and sample a partly-blended pose, so the unpadded range is the correct input here.
- `src/editor/timeline/CameraLane.tsx` does **not** call this: its span bar must follow a diamond mid-drag, so it pads its own drag-adjusted keyframe list (`kfs[0].t - KF_BLEND_MS` … `kfs[at end].t + KF_BLEND_MS`) inline instead of reading committed doc values.

## camMoveAt

```ts
export function camMoveAt(moves: CameraMove[], t: number, live?: CamPose | null): CamPose | null
```

Resolves the PiP pose at time `t`, or `null` when the keyframes do not own that frame.

### Inputs

- `moves: CameraMove[]` - `doc.camera_moves`, in any order. *Why sorted defensively:* the edit ops keep this array sorted by `t_ms`, but `camMoveAt` never trusts that from outside - it sorts a fresh copy (`[...moves].sort(...)`) rather than mutating the caller's array.
- `t: number` - the preview's current playhead time in milliseconds, same cadence as `camAt`/`layoutAt`.
- `live?: CamPose | null` - the LIVE layout-resolved PiP pose for THIS frame (derived from `PreviewLayout.cam`'s center + height, i.e. what `layoutAt` resolved before any override), or omitted/`null`. *Why per-frame, not a one-off static pose:* it is what the two blends ease to and from, and a layout cross-fade moves it while the exit blend is running. Omitted/`null` skips both blends (each snaps to the nearest end keyframe); the span rule itself never depends on it. Mirrors the Rust `sample`'s `live: Option<CamPose>`.

### Returns

- `null` if `moves` is empty, or `t` is outside `[first - KF_BLEND_MS, last + KF_BLEND_MS]` - the caller's signal to leave the PiP rect exactly as the layout resolved it (the "empty means default" model, same convention `layoutAt`'s gap-fallback uses).
- `CamPose` inside that span, per the five cases below.

### Implementation

1. Sort a copy of `moves` ascending by `t_ms` - `const ks = [...moves].sort((a,b) => a.t_ms - b.t_ms)`; empty -> `null`.
2. `entry = max(0, first.t_ms - KF_BLEND_MS)`; `t < entry || t > last.t_ms + KF_BLEND_MS` -> `null`. **Case 1.**
3. `t < first.t_ms` - **case 2, the entry blend:** `mix(live, pose(first), ease(first.easing, (t - entry) / (first.t_ms - entry)))`. Continuous at both ends (exactly `live` at `entry`, exactly the keyframe pose at `first`). No `live`, or a zero-length window (a keyframe at `t_ms = 0`) -> the first keyframe's pose.
4. `t > last.t_ms` - **case 4, the exit blend:** `mix(pose(last), live, ease(last.easing, (t - last.t_ms) / KF_BLEND_MS))`. No `live` -> the last keyframe's pose.
5. `t >= last.t_ms` -> the last keyframe's pose (also the single-keyframe instant, **case 5**: a lone keyframe eases in, hits its pose, eases back out - a hold needs two keyframes).
6. **Case 3, in-span - unchanged math:** find `bi`, the first index whose `t_ms` exceeds `t`, giving the straddling pair `a = ks[bi-1]`, `b = ks[bi]`. `b.t_ms === a.t_ms` (coincident keyframe times) -> `b`'s pose, guarding the division. Else `f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms))` - progress measured from entering `a` toward `b`, eased with `b`'s OWN easing curve, using the same `ease` (`src/editor/timeline/layoutTrack.ts`) `layoutAt` uses - no new easing implementation. *Why `b`'s easing, not `a`'s:* matches `CameraMoveTrack::sample` and `layoutAt`, which ease into a newly-entered segment using that segment's own easing, not the one being left.
7. Every blend and interpolation goes through the module-private `mix`, a component-wise lerp of `x`/`y`/`size`; `rectFromCenter` derives the rect from the RESULT once, so the panel's aspect handling applies to a blended pose too.

### Behaviors worth knowing

(Mirrors `moves_tests.rs` / `moves_span_tests.rs` - see `docs/api/src-tauri/src/export/camera/moves.md` for the Rust-side authority these correspond to. TS-side they are split the same way: `cameraMoves.test.ts` for the in-span math, `camMoveSpan.test.ts` for the span and blends.)

- Empty track -> `null` at any `t`.
- `linear` easing at the exact midpoint between two keyframes yields the arithmetic mean of `x`/`y`/`size` to within `1e-6`.
- At `t=250` of a `[0,1000]` `smooth`-eased span (`u=0.25`), the sampled `x`/`size` equal `ease("smooth", 0.25)` exactly, which is provably `!= 0.25` (smoothstep only equals its input at `0`, `0.5`, `1`).
- Two keyframes sharing the same `t_ms` never divides by zero; sampling exactly at that time returns one of the two poses, not a lerp.
- Keyframes passed in descending `t_ms` order still sample correctly, proving `camMoveAt`'s own sort (not caller discipline) is what keeps it correct - and the caller's original array is left in its original order.
- Keyframes at `2000`/`4000` sampled at `3000` reproduce the pre-Task-27 value (`x = 0.40`, pinned literally) and are identical with and without a `live` pose - `live` has no influence mid-span. Same numbers as the Rust `mid_span_interpolation_is_unchanged_by_the_span_rewrite`.
- With keyframes at `2000`/`4000` (span `[1650, 4350]`): `1000`/`5000` are `null`; `1649`/`4351` are `null` while `1650`/`4350` are not (the edges are inside); the entry blend equals `live` at `1650` and the keyframe pose at `2000` within `1e-4`, with the midpoint strictly between; and with a `live` pose sweeping linearly across the exit window, the blend at `4350` lands on `live(4350)` within `1e-4`, not the stale `live(4000)`.
- One keyframe at `3000` is a bump: `null` at `2649` and `3351`, its own pose at `3000`, and exactly `live` at both window edges.
- `KF_BLEND_MS` is asserted to be `350` in the TS test as well, so a change on one side without the other fails immediately.

### Used by

- `src/editor/stage/frameCam.ts` - `frameCamLayout` overrides the webcam rect fed to `drawPreview` when non-`null`, and falls through to the smart webcam-on-zoom action when `null` (which is now also the case outside the span); passes a `live` pose derived from `base.cam`.
- `src/editor/stage/Stage.tsx` - `sampledPose` (drag-handle positioning) passes the same `live` derivation so the handle matches what's drawn, and falls back to that live pose when the sample is `null`.
- `src/editor/panels/CameraPanel.tsx` - the Move-mode "Webcam size" slider reads the keyframed size at the playhead, falling back to the static `cam_size` wherever the sample is `null`.

## rectFromCenter

```ts
export function rectFromCenter(p: CamPose, ow: number, oh: number, aspect: number): [number, number, number, number]
```

TS mirror of `rect_from_center` (`src-tauri/src/export/scene/mod.rs`) - converts a sampled `CamPose` into a fraction-of-output `[x, y, w, h]` rect (top-left form), matching the export byte-for-byte.

### Inputs

- `p: CamPose` - the sampled pose (`camMoveAt`'s result).
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
- Radius is not part of this helper's output - the caller splices the existing (unoverridden) radius back in, scaled via `radiusScaleForResize` (Task 9 Part C) so the override composes correctly with a resize; "leave the camera panel radius as resolved AND SCALE IT" is the caller's contract, not this formula's.

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

- `overrideCamPanel` (below) and `src/editor/stage/Stage.tsx` (the drag handle's `pipRect`), so the handle, the drawn preview, and the export all agree on a Wide panel's shape.

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
- `p: CamPose`, `ow`/`oh: number` - same as `rectFromCenter`. The `aspect` is NOT a parameter here: it is derived from `baseCam` itself via `camAspect`, so the override can never disagree with the panel it is overriding (mirroring the renderer reading it off `scene.camera.rect`).

### Returns

A new 9-tuple: `rectFromCenter(p, ow, oh, camAspect(baseCam, ow, oh))`'s `[x,y,w,h]`, followed by `baseCam[4]` (radius) and `baseCam[5]` (ring width) each scaled by `radiusScaleForResize(baseCam[3], newRect[3])`, followed by `baseCam[6..9]` (ring color) unchanged. Mirrors the Rust `Panel { rect, radius: radius*m, ring_px: ring_px*m, ..panel }` - alpha and ring color are never touched by the override, only rect/radius/ring-width scale.

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - replaces `baseLayout.cam` with `overrideCamPanel(baseLayout.cam, cp, c.width, c.height)` when a camera_moves/drag pose is active and a camera panel is resolved this frame, so the preview's PiP rect, radius, AND ring stay in parity with the export's `override_camera` after a keyframe resize.
