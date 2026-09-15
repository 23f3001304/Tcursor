# src/editor/director/review/outline.ts

Projects a proposal's region onto the stage. The region is `[x, y, w, h]` in 0..1 fractions of the screen CONTENT - the same basis `ZoomTarget::Fixed` stores and the same one `mapCanvasClickToZoomTarget` produces - so it has to go through the live camera crop before it means anything in pixels.

## OutlineBox

```ts
export interface OutlineBox { left: number; top: number; width: number; height: number }
```

One box as 0..1 fractions of the `.e-stage` box, which is exactly the canvas' displayed box, so the caller converts straight to CSS percentages.

## outlineRect

```ts
export function outlineRect(
  rect: [number, number, number, number], canvasW: number, canvasH: number,
  layout: PreviewLayout | null, cam: StageCam): OutlineBox | null
```

Maps the rect's two opposite corners through `mapZoomTargetToCanvasPoint` (`stage/camera/zoomTargetMapper.ts`), the exact forward of the click mapping the stage already uses, and returns their bounding box. Because it re-projects every render, the outline tracks the picture as a zoom ramps instead of floating over it.

**`null` when the box lies entirely outside `[0, 1]`.** The crop has pushed the region off frame, and a box clamped to an edge would point at something the region is not. A box only PARTLY off frame is returned unchanged and clipped by the stage's own `overflow: hidden` - that is the truth, since the region really does continue past the edge.

**Degenerate input is `null` too**: a non-finite number, or a zero/negative width or height, which is what a validated-but-empty rect would look like.

**Rounded to four decimals.** That is 0.13px on a 1280px stage, well under a pixel, and it keeps the float tail a subtraction leaves behind (`0.6 - 0.2 = 0.4000000000000001`) out of the DOM and out of equality comparisons.

### Used by

- `src/editor/stage/StageOutline.tsx` - the only caller; it samples the camera track for `cam` and paints the result.
