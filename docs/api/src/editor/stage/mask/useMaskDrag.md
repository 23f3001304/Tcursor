# src/editor/stage/mask/useMaskDrag.ts

The stage's mask drag box: the pure rect arithmetic, and the hook that turns pointer events into one `update_effect`.

**The exclusivity rule.** This is the FOURTH exclusive stage pointer mode, beside aim, Move and arrange. It is live only when `sel` names a mask region AND `arrangeSeg === null` AND `!moveMode` AND `!aimMode`, which is the same shape `Stage.tsx` already uses to gate `CamDragHandle` (`p.moveMode && !arranging`) and `ArrangeOverlay` (`p.arrangeSeg && arrange.panels`). Two overlays that both claim the pointer would fight over the same drags, so each mode's gate names every other mode.

**No drag logic lands in `Stage.tsx`.** The overlay is one sibling element there and the hook is one call in `useStageEngine`; everything else is in this file and `MaskOverlay.tsx`.

## Rect

```ts
export type Rect = [number, number, number, number];
```

`[x, y, w, h]` in CANVAS FRACTIONS, the shape `EffectRegion.rect` is stored in, which is why nothing here converts to pixels except to read a pointer delta.

## MaskHandle

```ts
export type MaskHandle = "move" | "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se";
```

Which grip is being dragged. The compass names are not decorative: `resizeMaskRect` tests them with `includes`, so a corner is literally its two edges and needs no case of its own.

## SNAP_TOL

```ts
export const SNAP_TOL = 0.01;
```

How close, in canvas fractions, an edge or a centre must come to a guide line before it snaps. One percent of the frame, the same feel as the arrange overlay's snapping.

## clampMaskRect

```ts
export function clampMaskRect(r: Rect): Rect | null
```

The TypeScript twin of `edit::ops::effects::clamp_rect`: sizes clamped to `[0.01, 1]`, position clamped so the rect stays wholly on the canvas. The one percent floor is what stops a rect being dragged to nothing and becoming unfindable.

*Why it returns `null` on a non-finite rect* rather than repairing it: the Rust side's rule is "keep the stored one" when an update is not a usable rect, and `null` is how that decision reaches the caller. `useMaskDrag` then ignores that pointer move entirely, so the draft holds its last good value instead of jumping to a `NaN` box.

## resizeMaskRect

```ts
export function resizeMaskRect(r: Rect, handle: MaskHandle, dx: number, dy: number): Rect
```

Applies a fractional delta to one handle. `move` translates and leaves the size alone; `w` and `n` move an edge AND shrink the size by the same amount, so the opposite edge stays put; `e` and `s` only grow. A corner runs both of its edges' branches.

The result may be inverted or off-canvas: this function does the geometry and `clampMaskRect` does the policy, so the two can be read and tested apart.

## snapMaskRect

```ts
export function snapMaskRect(r: Rect, tol: number): { rect: Rect; guideX: number | null; guideY: number | null }
```

Snaps each axis independently to three lines - the two frame edges and the centre, `0`, `0.5`, `1` - testing the rect's near edge, far edge and CENTRE against each, in that order, and returns the guide that was hit so the overlay can draw it.

*Why the centre is a snap target and not just the edges:* centring a highlight or a blur on the frame is the single most common placement, and doing it by eye at one pixel of tolerance is not possible.

**A guide can be reported without the rect moving.** A rect that is already aligned on an axis snaps to where it already is, and the guide still fires - which is the point, since the line is how the user knows they are aligned. `reports a guide on an axis that is already aligned without moving it` pins exactly that.

## useMaskDrag

```ts
export function useMaskDrag(args): { box: MaskPx | null; guideX: number | null; guideY: number | null;
                                     onHandleDown: (e: React.PointerEvent, h: MaskHandle) => void }
```

Returns the box to draw and the grip handler. `box` is `maskDraws` run over the selected region, with the live draft rect substituted while a drag is in flight, so the overlay is placed by exactly the projection that places the mask itself and cannot drift from it.

**The pixel-to-fraction conversion is exact, and comes from the projection.** `onHandleDown` maps canvas fractions `(0,0)` and `(1,1)` through `fxFrameGeometry` and keeps the span between them: because that map is affine, that span IS the canvas pixels per unit fraction on each axis, so a pointer delta divided by it is a fraction delta whatever the zoom, the layout or the display span happens to be doing. There is no second copy of the geometry to keep in step. The further division by the element's CSS scale (`.e-canvas`'s laid-out width over its backing-store width) is because the canvas is displayed at a different size than it is drawn at.

**One `update_effect` on release, not one per move.** The drag is local state until `pointerup`, so it is ONE undo step and one document write rather than one per frame of the drag. `pastDragThreshold` gates the commit, so a click that merely selects the box writes nothing at all.

**The identity camera is deliberate.** The hook is passed `{ cx: 0.5, cy: 0.5, scale: 1 }` rather than the live camera, because the stage's canvas ALREADY shows the camera's crop; applying the camera a second time here would double it and the box would drift away from the mask under any zoom. If the owner's look pass finds the box lagging the picture during a live zoom, the fix is to pass `camAt(trackRef.current, tOut)` here and change nothing else.

### Used by

- `src/editor/stage/useStageEngine.ts` - one call, gated on the three other pointer modes, returning `mask` for `Stage` to render.
