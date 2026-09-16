# src/editor/stage/canvas/cursorLayer.ts

The cursor's own layer of the editor preview, split out of `previewCanvas.ts` so there is a real seam between the panels and the cursor - `drawCompositeFrame` (`hooks/stage/compositeFrame.ts`) calls this right after `drawPreview` returns its `PreviewGeom`, and Batch 2's `drawMasks`/`gradeCanvas` land in the gap between the two calls. Draws through `drawCursorSprite` (`../cursor/cursorPreview.ts`) at the cursor's zoom-projected position, scaled and clipped to the screen panel exactly as `drawPreview` scales and clips the screen and webcam panels - matching the export, which projects/scales the cursor after its own crop too.

## drawCursorLayer

```ts
export function drawCursorLayer(
  ctx: CanvasRenderingContext2D, w: number, h: number, cam: DrawCam,
  cursor: DrawCursor | null, clicks: ClickSample[], now: number,
  insetW: number, g: PreviewGeom
): void
```

The cursor blit `drawPreview` used to make, on the far side of the seam: takes the `PreviewGeom` `drawPreview` (`previewCanvas.ts`) returned instead of recomputing the panel rect and crop itself, so it draws against the exact frame the panels - and, once Batch 2 lands, the mask and the grade - actually painted.

### Inputs

- `ctx`, `w`, `h` - the same 2D context and canvas size passed to `drawPreview` for this frame.
- `cam: DrawCam` (`previewCanvas.ts`) - the interpolated camera pose; only `curx`/`cury` are read here, to project the cursor's panel-local position through `g.crop`.
- `cursor: DrawCursor | null` - the cursor inputs, or `null` to skip. *Why:* drawn (Enhanced only) at the zoom-projected cursor position with bounce + motion trail, scaled by the screen panel's own size and clipped to its on-screen rect (see `insetW`) - matching the export, which doesn't scale cursor size with the CAMERA zoom either.
- `clicks: ClickSample[]`, `now: number` - the click track and current output time (ms). *Why:* passed through to `drawCursorSprite` for its click-bounce dip animation (ripples themselves are drawn by the backend FX overlay, not here).
- `insetW: number` (fraction of `w`) - `LayoutPresets.inset_w`, the export's fixed reference width for the cursor's scale-down (`cursorset::draw`'s `panel` factor; see `cursorPanel.ts`). *Why:* the caller passes `LayoutPresets.inset_w ?? 1`, a neutral "no shrink" fallback for the brief window before layout presets have loaded, same spirit as `drawPreview`'s `layout: null` inset fallback.
- `g: PreviewGeom` - the geometry `drawPreview` (`previewCanvas.ts`) returned this frame.

### Returns

`void` - draws directly onto `ctx`, or touches it not at all when the early return below fires.

### Implementation

Skip entirely when `!cursor`, or `!g.hasVideo`, or `g.screenAlpha < 0.5` - mirroring `cursorset::draw`'s own gate (`if screen.alpha < 0.5 { return; }`): the export stops drawing the synthetic cursor once the screen panel is more than half faded out, so a cross-fade into `camera_only` must not leave a cursor hanging over a panel that is no longer there. Otherwise: project the cursor's panel-local position through `g.crop`; derive `panel = panelFactor(dw / w, insetW)` and `clip = panelClipRect({x:dx,y:dy,w:dw,h:dh}, {cx0,cy0,cw,ch}, w, h)` (`cursorPanel.ts`) from `g.panel` and `g.crop`, plus `capturedScale = contentScale(panel, insetW * w, (cursor.captured?.srcW ?? 0) * g.src[2])` for the captured cursor, and pass all three to `drawCursorSprite`. *Why a third number:* the synthetic sprites are authored against the output canvas and scale by `panel`, while the CAPTURED bitmaps are in the recording's own source pixels and have to shrink with the content (`insetW` is a canvas fraction, scaled up here to share `srcW`'s unit) - `srcW` itself is scaled by `g.src[2]` first, matching Rust passing `Scene.src.w` into `captured::content_scale`.
