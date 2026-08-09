# src/editor/stage/zoomTargetMapper.ts

Converts **both ways** between a canvas position and the 0..1 screen-content fraction that `update_zoom`'s `Fixed` (Region) target stores, so clicking the preview targets the same point the user sees - even when a zoom is already active (un-projecting through its current crop) - and so a stored target can be drawn back onto the stage as the aim reticle.

Both directions read the panel rect + zoom crop from one private helper (`stageCrop`), which is the single place `drawPreview`'s projection model is mirrored. `zoomTargetMapper.test.ts` pins them as exact inverses (round-trip to 5-6 decimal places at scale 1 and at scale 2.5 with an off-centre camera).

## mapCanvasClickToZoomTarget

```ts
export function mapCanvasClickToZoomTarget({
  clientX, clientY, canvasElement, layout, cam,
}: {
  clientX: number; clientY: number;
  canvasElement: { width, height, getBoundingClientRect() };
  layout: PreviewLayout | null; cam: StageCam;
}): [number, number] | null
```

`canvasElement` is typed structurally (not as `HTMLCanvasElement`) purely so the round-trip test can pass a plain object with a fixed bounding rect; a real `<canvas>` satisfies it unchanged.

### Inputs

- `clientX`, `clientY` - the click's viewport coordinates (from the DOM event).
- `canvasElement` - the preview `<canvas>`; its bounding rect converts client coords to backing-store pixels, its `width`/`height` are the projection's frame size.
- `layout: PreviewLayout | null` - the screen panel rect (fractions), or `null` for the inset fallback - must match what `drawPreview` used to render the frame being clicked.
- `cam: { scale, cx, cy }` - the camera pose active *right now*, so the click is un-projected through whatever zoom crop is currently on screen.

### Returns

`[number, number] | null` - a 0..1 screen-content fraction, or `null` when the click landed outside the screen panel (e.g. on the background or webcam PiP).

### Implementation

1. Convert the click to canvas backing-store pixels via `canvasElement.getBoundingClientRect()`.
2. Resolve the screen panel rect (`dx,dy,dw,dh`) from `layout` or the inset fallback - identical math to `drawPreview`.
3. Recompute the *same* whole-frame zoom crop `(cx0,cy0,cw,ch)` `drawPreview` used for this `cam`.
4. Un-project the click through that crop back to a pre-zoom, panel-local point; reject it (`null`) if it falls outside the screen rect.
5. Return `(point - panel origin) / panel size` as the 0..1 fraction.

### Notes

- This function only needs to stay consistent with `drawPreview`'s zoom-crop math (in `previewCanvas.ts`) - if that projection model changes, this one must change with it, or "click to add a zoom while already zoomed in" will target the wrong point.

## mapZoomTargetToCanvasPoint

```ts
export function mapZoomTargetToCanvasPoint({ tx, ty, canvasW, canvasH, layout, cam }: {
  tx: number; ty: number; canvasW: number; canvasH: number;
  layout: PreviewLayout | null; cam: StageCam;
}): [number, number]
```

The exact forward of `mapCanvasClickToZoomTarget`: given a stored zoom target (`tx`, `ty` - 0..1 screen-content), returns where it lands on the canvas **right now**, as 0..1 fractions of the canvas' displayed box.

### Returns

Fractions, **not clamped**. Values outside `0..1` mean the aim point is currently cropped out of frame (e.g. a strong zoom parked elsewhere); the caller decides whether to draw it anyway - `.e-stage` has `overflow: hidden`, so `Stage` simply lets it clip.

### Notes

- `.e-stage`'s box IS the canvas' displayed rect (the stage is sized to the resolved aspect ratio, which the canvas fills exactly - no letterbox gap), so these fractions convert straight to CSS percentages with no client-rect math. That is how `ZoomReticle` positions itself.
- It takes `canvasW`/`canvasH` rather than the element, because the caller (`Stage`) already has the resolved backing-store size and the reticle is placed in *stage* percentages, never in client pixels.

## zoomTargetPoint

```ts
export function zoomTargetPoint(target: ZoomTarget | undefined | null): [number, number] | null
```

The aim point of a `ZoomTarget`: the stored `fixed` pair for a **Region** target, or `null` when it follows the live cursor (which has no stored point) or when there is no target at all (nothing selected).

`Editor` uses the `null` as a three-way signal: no reticle, no aim mode possible, and the canvas click keeps its normal add-a-zoom meaning.
