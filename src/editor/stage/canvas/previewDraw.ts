export function paintPanel(
  target: CanvasRenderingContext2D,
  alpha: number,
  scratch: HTMLCanvasElement,
  w: number,
  h: number,
  draw: (c: CanvasRenderingContext2D) => void,
) {
  if (alpha >= 0.999) {
    draw(target);
    return;
  }
  if (scratch.width !== w) scratch.width = w;
  if (scratch.height !== h) scratch.height = h;
  const sc = scratch.getContext("2d");
  if (!sc) {
    draw(target);
    return;
  }
  sc.clearRect(0, 0, w, h);
  draw(sc);
  target.save();
  target.globalAlpha = alpha;
  target.drawImage(scratch, 0, 0);
  target.restore();
}

export function coverDraw(
  ctx: CanvasRenderingContext2D,
  img: CanvasImageSource,
  sw: number,
  sh: number,
  dx: number,
  dy: number,
  dw: number,
  dh: number,
) {
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale,
    ch = dh / scale;
  ctx.drawImage(img, (sw - cw) / 2, (sh - ch) / 2, cw, ch, dx, dy, dw, dh);
}

export function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
) {
  const rr = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.arcTo(x + w, y, x + w, y + h, rr);
  ctx.arcTo(x + w, y + h, x, y + h, rr);
  ctx.arcTo(x, y + h, x, y, rr);
  ctx.arcTo(x, y, x + w, y, rr);
  ctx.closePath();
}
