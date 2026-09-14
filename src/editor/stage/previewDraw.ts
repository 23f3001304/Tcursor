/** The canvas-drawing primitives `previewCanvas.ts` composites with: one-shot panel compositing,
 *  cover-fit drawing and a rounded-rect path. Pure drawing, no layout or timing - split out of
 *  `previewCanvas.ts` for its line budget so that file stays the frame it composites. */

/** Composite one panel ONCE at `alpha`, the way the export's shader does - a single
 *  `mix(color, panel, cov * panel_a)` per panel (gpu/shader.wgsl). Setting `globalAlpha` and then
 *  drawing the panel's own layers straight onto `target` composites EACH of them separately: the
 *  shadow backing and the video both land at `alpha`, so a panel at alpha 0.5 came out as
 *  `0.5*video + 0.25*bg + 0.25*black` - visibly DARK through every layout cross-fade, and worst at
 *  the middle of it, which the export never does. Painting them into a scratch layer and blitting
 *  that once is the same picture the shader produces (plus the preview's decorative shadow, which
 *  now fades WITH its panel instead of as a layer of its own).
 *
 *  Opaque is the overwhelmingly common case and takes the direct path, so the extra full-canvas
 *  buffer is only ever touched on the frames a transition is actually running. */
export function paintPanel(target: CanvasRenderingContext2D, alpha: number, scratch: HTMLCanvasElement,
  w: number, h: number, draw: (c: CanvasRenderingContext2D) => void) {
  if (alpha >= 0.999) { draw(target); return; }
  if (scratch.width !== w) scratch.width = w;
  if (scratch.height !== h) scratch.height = h;
  const sc = scratch.getContext("2d");
  if (!sc) { draw(target); return; } // no context: an undimmed panel beats no panel
  sc.clearRect(0, 0, w, h);
  draw(sc);
  target.save();
  target.globalAlpha = alpha;
  target.drawImage(scratch, 0, 0);
  target.restore();
}

/** Draw `img` to cover the dest rect (centre-crop the source to the dest aspect). */
export function coverDraw(ctx: CanvasRenderingContext2D, img: CanvasImageSource, sw: number, sh: number,
  dx: number, dy: number, dw: number, dh: number) {
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale, ch = dh / scale;
  ctx.drawImage(img, (sw - cw) / 2, (sh - ch) / 2, cw, ch, dx, dy, dw, dh);
}

export function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.arcTo(x + w, y, x + w, y + h, rr);
  ctx.arcTo(x + w, y + h, x, y + h, rr);
  ctx.arcTo(x, y + h, x, y, rr);
  ctx.arcTo(x, y, x + w, y, rr);
  ctx.closePath();
}
