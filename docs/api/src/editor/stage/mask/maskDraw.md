# src/editor/stage/mask/maskDraw.ts

Paints the masks `maskPreview::maskDraws` projected onto the stage's own 2D canvas, in the seam between the panels and the cursor sprite. The export's equivalents are `fx_mask.wgsl` on the GPU and `maskdraw.rs` on the CPU; this is a third implementation because a 2D canvas has neither, and it reaches the same look through canvas primitives instead.

## drawMasks

```ts
export function drawMasks(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, masks: MaskPx[]): void
```

Draws each mask in order, skipping any at alpha 0. `c` is the canvas `ctx` belongs to, passed explicitly because blur and pixelate both re-read the canvas they are drawing onto.

### The three kinds

- **Blur** clips to the rounded rect, sets `ctx.filter = blur(sigma)` with the sigma from `blurSigmaFor`, and draws the canvas back onto itself. The self-draw is what makes it a blur OF the picture rather than a blurred copy of something else.
- **Pixelate** downsamples the rect into an offscreen canvas of `ceil(size / cell)` pixels with smoothing ON, then draws it back up with `imageSmoothingEnabled = false` so the cells come back as hard squares. Sampling down with smoothing is what averages each cell; drawing up without it is what keeps the cell edges crisp.
- **Highlight** opens a path with `ctx.rect` over the whole canvas, lets `roundRect` add the mask rect as a second subpath, and fills with `evenodd` so only the OUTSIDE is darkened. `roundRect` calls `ctx.beginPath()` itself, which is why the full-canvas rect is added first and the hole second.

### The stated approximations

Two, both named on `maskPreview.md` beside the third (the corner rounding):

1. **No feather.** `clipTo` is a hard clip; a 2D clip path has no soft edge and `ctx.filter` applied to one is not portable across browsers. The mask's `featherPx` is carried but not drawn, so a soft-edged mask reads as hard-edged on the live stage.
2. **A CSS Gaussian, not three box passes.** Matched through `blurSigmaFor`, so the two are the same strength even though they are not the same kernel.

**The paused exact frame is the truth.** `useExactFrame` stamps the real rendered frame over the canvas when the playhead is parked, so the feather, the exact kernel and the true stacking are all visible there. That has been the standing visual gate since 2026-09-14, and it is why these approximations are acceptable on the live scrub.

**One difference from the export that is NOT an approximation:** here every mask reads the running canvas, where `maskdraw::draw_masks` snapshots the frame once and both paths sample that. Two OVERLAPPING masks therefore compose differently on the live stage than in the export. Non-overlapping masks, which is every ordinary use, are identical.

### Used by

- `src/editor/hooks/stage/compositeFrame.ts` (`drawCompositeFrame`) - the one call site, after the panels and before the cursor.
