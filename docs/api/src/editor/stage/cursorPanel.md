# src/editor/stage/cursorPanel.ts

The export's synthetic cursor scales AND clips to the screen panel (`cursorset.rs`: "scaled to the panel size ... and clipped to the panel's on-screen rect so it never spills onto the background or the webcam"). Pulled out of `previewCanvas.ts` as pure ratio/projection math - no canvas, no DOM - so this exact mirror is unit-tested directly rather than eyeballed on screen. Same extraction pattern as `fxGeometry.ts` / `pointerGesture.ts`.

## panelFactor

```ts
export function panelFactor(screenW: number, insetW: number): number
```

The cursor's scale-down factor for a screen panel of width `screenW`, against the export's fixed reference inset width `insetW` (`LayoutPresets.inset_w` - a baseline independent of the active preset/arrangement, NOT the panel's own preset rect).

### Inputs

- `screenW: number` - the active screen panel's own width.
- `insetW: number` - the export's reference inset width (`LayoutPresets.inset_w`). Both args must be the same unit (canvas px, or a fraction of the canvas - it's a pure ratio, so either works); `previewCanvas.ts` passes fractions since that's what `PreviewLayout.screen`/`LayoutPresets.inset_w` already are.

### Returns

`number` - `(screenW / max(insetW, 1e-6)).clamp(0.1, 1.0)`, mirroring `cursorset.rs`'s `(screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0)` exactly, except the divide-by-zero floor is a tiny epsilon rather than "1 output px" (which has no fixed meaning in fraction units) - it only ever matters for a degenerate zero `insetW`, never a real one.

### Used by

- `src/editor/stage/previewCanvas.ts` - `drawPreview` computes `panelFactor(dw / w, insetW)` from the screen panel's own pre-zoom width and passes it to `drawCursorSprite`.

## ZoomCrop

```ts
export interface ZoomCrop { cx0: number; cy0: number; cw: number; ch: number }
```

The zoom crop `previewCanvas.ts` resizes the whole base frame through (its `cx0`/`cy0`/`cw`/`ch`) - same shape as the export's `coordmap::crop`.

## panelClipRect

```ts
export function panelClipRect(
  rect: { x: number; y: number; w: number; h: number },
  crop: ZoomCrop,
  canvasW: number, canvasH: number,
): [number, number, number, number]
```

Projects `rect` (pre-zoom canvas px, the offscreen base-frame space `drawPreview` composites into - e.g. the screen panel's own `dx`/`dy`/`dw`/`dh`) through `crop` the same way a point is mapped to its on-screen position, then clamps to the canvas bounds. Mirrors `cursorset.rs`'s `project_rect` (in turn `coordmap::project` on each corner + `coordmap::crop`) - the box the export confines the cursor AND its motion trail to, so neither ever spills onto the background or the webcam.

### Inputs

- `rect` - the panel's own pre-zoom rect.
- `crop: ZoomCrop` - the active zoom crop.
- `canvasW`, `canvasH` - the canvas size to clamp into.

### Returns

`[number, number, number, number]` - post-zoom canvas px `[x0, y0, x1, y1]`. Deliberately NOT normalized when the panel falls entirely outside the crop (`x0` can end up past `x1`) - exactly like the Rust tuple, which relies on its caller's `ox_start >= ox_end` no-op check rather than reordering; `drawCursorSprite` carries the same guard before it draws anything.

### Used by

- `src/editor/stage/previewCanvas.ts` - `drawPreview` computes `panelClipRect({x:dx,y:dy,w:dw,h:dh}, {cx0,cy0,cw,ch}, w, h)` from the same panel rect and crop `drawPreview` already has, and passes it to `drawCursorSprite`.
- `src/editor/stage/cursorPreview.ts` - `drawCursorSprite` no-ops when the result is empty/inverted, and clips the trail + main blit to it otherwise.
