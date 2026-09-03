# src/editor/stage/arrange/arrangeMath.ts

The pure half of stage arrange mode (T34 L3): pointer deltas to a `PanelPose`, corner resizing, snapping, and the two small presentation/op builders the gesture needs. No React, no DOM - every function here is a plain value transform, and every one is pinned by `arrangeMath.test.ts`.

**Where the pose-math boundary sits.** Resolution - a pose to a real panel rect with radius/ring/alpha - stays in Rust (`export::scene::arrangement`), surfaced to the preview as already-resolved rects (`LayoutPresets.segs`, see `layoutTrack.md`). What lives here is only the interaction: where the pointer went, and which pose that means. The two places that touch geometry beyond that (`poseOfRect`, `draftPanels`) are the inverse of `rect_from_center` and the existing tested `radiusScaleForResize` mirror respectively - both needed so a drag can start from the rect the user actually grabbed and preview at pointer rate, both discarded the moment Rust re-resolves the committed pose.

## MIN_SIZE

```ts
export const MIN_SIZE = 0.05, MAX_SIZE = 1.5;
```

The `PanelPose.size` range, mirroring the Rust op clamps (`edit/ops/arrangement.rs`) so a drag can never build a pose the backend would silently clamp out from under the live preview. `cx`/`cy` clamp to `0..1` the same way.

## SNAP_PX

```ts
export const SNAP_PX = 8;
```

The snap threshold, in STAGE pixels - literally 8 pixels on screen. `useArrangeDrag` converts it against the canvas' **displayed** rect (`getBoundingClientRect`), not its backing store, since CSS scales the stage to fit its box: at a 1280-wide backing store shown at 640 CSS px, converting against the backing store would have made the threshold feel like 4px. Measured once per gesture (the stage cannot resize mid-drag), so it stays off the pointermove path.

## SnapTol

```ts
export type SnapTol = [number, number];
```

That threshold as a per-axis tolerance in FRACTIONS of the output frame, which is the currency the pure functions work in. `null` means snapping is off - it is what Alt produces, so the "is snapping on" flag and the "how close counts" basis are one parameter that cannot disagree.

## SNAP_TARGETS

```ts
export const SNAP_TARGETS = [0.045, 1 / 3, 0.5, 2 / 3, 1 - 0.045];
```

The snap lines, as fractions of the output frame - the same set on both axes: the 4.5% safe margins, the thirds, and the frame centre. A MOVE tests the panel's near edge, centre and far edge against them; a RESIZE tests the dragged corner's two edges.

## VISIBLE_ALPHA

```ts
export const VISIBLE_ALPHA = 0.004;
```

Alpha at or below which a resolved panel counts as hidden - the same threshold `toPreviewLayout` (`../../timeline/layoutTrack.ts`) uses to decide whether to draw the cam at all. Read by the overlay (which frames only visible panels), the inspector (its show/hide switches) and `setArrangementOp`.

## poseOfRect

```ts
export const poseOfRect: (r: Rect) => PanelPose
```

The pose a resolved rect stands for: `{ cx: x + w/2, cy: y + h/2, size: h }` - the inverse of the Rust `rect_from_center`.

Every gesture starts from this rather than from the segment's stored `arrangement`, so what the user grabbed is what gets committed even on a segment that has no arrangement yet (its rect comes from the provenance preset instead). The two agree to within the 0.33px `inset_rect` rounding residual documented in the L1 parity table.

## rectAspect

```ts
export const rectAspect: (r: Rect, ow: number, oh: number) => number
```

A panel's own pixel aspect (w/h) from a resolved rect whose `w`/`h` are fractions of DIFFERENT axes. This is the `aspect` `rectFromCenter` needs so a pose - which carries height only - can never stretch the panel. Generic over both panels (`camAspect` in `cameraMoves.ts` is the same computation, typed to the `PreviewLayout.cam` tuple).

## movedPose

```ts
export function movedPose(start: PanelPose, d: [number, number], aspect: number,
  ow: number, oh: number, snap: boolean): DragOut
```

Body drag: the start pose translated by a pointer delta in frame fractions, clamped into the frame, then (unless `snap` is false) snapped.

Snapping runs per axis and considers three candidate lines on each - the panel's near edge, its centre and its far edge - taking whichever lands closest to a target inside the tolerance and shifting the CENTRE by that delta.

The guides are then **re-derived from the final, clamped pose** (a half-output-pixel hairline against the same targets), not reported straight from the snap: pressing a panel against the frame edge can leave the snap wanting a centre outside `0..1`, and without this the overlay would draw a line through a target the panel had been clamped back off. Same rule `resizedPose` follows for the same reason. An axis sitting on nothing reports `null`.

## resizedPose

```ts
export function resizedPose(r: Rect, c: Corner, ptr: [number, number], aspect: number,
  ow: number, oh: number, snap: boolean): DragOut
```

Corner drag: the panel resized about its OPPOSITE corner.

The height comes from projecting the anchor-to-pointer vector (in output pixels, so the aspect is meaningful) onto the panel's own aspect diagonal `(aspect, 1)`, so the pointer can leave the diagonal freely and the panel still grows along it: **width always follows the aspect, and content is never stretched.** The new centre is then placed so the anchor corner stays exactly where it was, and the size clamps to `MIN_SIZE..MAX_SIZE` - dragging through the anchor, or far past the frame, both land on a clamp rather than inverting the rect.

Snapping here changes SIZE, not centre (the anchor is fixed): each candidate target is solved back into a height about the anchor, and the one whose corner lands nearest the unsnapped corner wins. The guides are then recomputed from the CLAMPED pose, so a size the clamp pulled off its target does not keep drawing a guide it no longer touches.

*(Known, filed rather than fixed: `snapResizeHeight` solves each candidate with `Math.abs`, so a target on the far side of the anchor would map to a mirrored height. Unreachable in practice - the pointer would have to be dragged through the anchor, where the `MIN_SIZE` clamp already owns the result.)*

## draftPanels

```ts
export function draftPanels(base: Panels, panel: PanelKind, rect: Rect): Panels
```

The live-drag panel pair: `base` with ONE panel's rect replaced and forced visible.

The cam's radius and ring scale by the same height ratio `override_camera` applies (through the existing tested `radiusScaleForResize` mirror), so a circular webcam stays round mid-drag; the screen's radius is a fraction of canvas HEIGHT and so does NOT scale with its panel, matching how L1's resolution treats the two. Ratios compose, so scaling from an already-posed entry gives the same result as scaling from the preset.

Presentation only - the committed pose is re-resolved by Rust the moment the op lands.

## withDraftSeg

```ts
export function withDraftSeg(presets: LayoutPresets, segId: string, p: Panels): LayoutPresets
```

`presets` with one segment's `segs` entry replaced (or appended when it has none yet), input untouched.

This is what keeps arrange mode from needing a drawing path of its own: the draft goes back through the exact per-segment resolved-rect channel L2 built, so `layoutAt` picks it up like a committed arrangement - cross-fades into and out of the dragged segment included - and `Stage` only has to hand the composite loop the merged presets instead of the raw ones.

## setArrangementOp

```ts
export function setArrangementOp(seg: LayoutSeg, base: Panels, panel: PanelKind,
  pose: PanelPose | null): EditOp
```

The `set_arrangement` op for changing ONE panel (`pose: null` hides it).

On a segment that already has an arrangement, only the touched panel's key goes on the wire - key absent means "leave that panel alone" (L1's wire table). On a segment with NO arrangement the backend bases the write off "both panels hidden", so the untouched panel is filled in from what it resolves to right now: its current pose when visible, an explicit `null` when the provenance preset hides it. That makes a first drag - or a first cam-hide - preserve the rest of the segment's look instead of blanking half the stage, and it guarantees the write is never the "hide both panels" case the backend rejects.
