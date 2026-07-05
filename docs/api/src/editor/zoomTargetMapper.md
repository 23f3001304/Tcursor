# src/editor/zoomTargetMapper.ts

Inverts a canvas click into the 0..1 screen-content fraction that `update_zoom`'s `Fixed` target expects, so clicking the preview to add a new zoom targets the same point the user sees - even when a zoom is already active (un-projecting through its current crop).

## mapCanvasClickToZoomTarget

```ts
export function mapCanvasClickToZoomTarget({
  clientX, clientY, canvasElement, layout, cam,
}: {
  clientX: number; clientY: number; canvasElement: HTMLCanvasElement;
  layout: PreviewLayout | null; cam: { scale: number; cx: number; cy: number };
}): [number, number] | null
```

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
