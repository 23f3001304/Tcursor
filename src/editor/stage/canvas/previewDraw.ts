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

export interface ScreenPanelGeom {
  dx: number;
  dy: number;
  dw: number;
  dh: number;
  r: number;
  w: number;
  h: number;
  alpha: number;
  src: [number, number, number, number];
}

export interface ScreenMix {
  video: HTMLVideoElement;
  alpha: number;
}

export function drawScreenPanel(
  octx: CanvasRenderingContext2D,
  layer: HTMLCanvasElement,
  g: ScreenPanelGeom,
  screen: HTMLVideoElement,
  mix: ScreenMix | null,
) {
  const vw = screen.videoWidth,
    vh = screen.videoHeight;
  if (!(vw > 0 && vh > 0) || g.alpha < 0.004) return;
  const frame = (c: CanvasRenderingContext2D, v: HTMLVideoElement) =>
    c.drawImage(v, g.src[0] * vw, g.src[1] * vh, g.src[2] * vw, g.src[3] * vh, g.dx, g.dy, g.dw, g.dh);
  paintPanel(octx, g.alpha, layer, g.w, g.h, (c) => {
    c.save();
    c.shadowColor = "rgba(0,0,0,.5)";
    c.shadowBlur = 34;
    c.shadowOffsetY = 14;
    roundRect(c, g.dx, g.dy, g.dw, g.dh, g.r);
    c.fillStyle = "#000";
    c.fill();
    c.restore();
    c.save();
    roundRect(c, g.dx, g.dy, g.dw, g.dh, g.r);
    c.clip();
    if (mix && mix.video.videoWidth > 0) {
      frame(c, mix.video);
      c.globalAlpha = Math.min(1, Math.max(0, mix.alpha));
    }
    frame(c, screen);
    c.globalAlpha = 1;
    c.restore();
  });
}
