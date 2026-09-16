import { roundRect } from "../canvas/previewDraw";
import { blurSigmaFor, type MaskPx } from "./maskPreview";

function clipTo(ctx: CanvasRenderingContext2D, m: MaskPx) {
  const [w, h] = [m.mx[0] - m.mn[0], m.mx[1] - m.mn[1]];
  roundRect(ctx, m.mn[0], m.mn[1], w, h, m.r);
  ctx.clip();
}

export function drawMasks(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, masks: MaskPx[]) {
  for (const m of masks) {
    if (m.alpha <= 0) continue;
    if (m.kind === 3) highlight(ctx, c, m);
    else if (m.kind === 1) blur(ctx, c, m);
    else pixelate(ctx, c, m);
  }
}

function blur(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, m: MaskPx) {
  ctx.save();
  clipTo(ctx, m);
  ctx.globalAlpha = m.alpha;
  ctx.filter = `blur(${blurSigmaFor(m.amountPx).toFixed(2)}px)`;
  ctx.drawImage(c, 0, 0);
  ctx.restore();
}

function pixelate(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, m: MaskPx) {
  const [w, h] = [m.mx[0] - m.mn[0], m.mx[1] - m.mn[1]];
  const cell = Math.max(2, m.amountPx);
  const sw = Math.max(1, Math.ceil(w / cell));
  const sh = Math.max(1, Math.ceil(h / cell));
  const scratch = document.createElement("canvas");
  scratch.width = sw;
  scratch.height = sh;
  const sc = scratch.getContext("2d");
  if (!sc) return;
  sc.imageSmoothingEnabled = true;
  sc.drawImage(c, m.mn[0], m.mn[1], w, h, 0, 0, sw, sh);
  ctx.save();
  clipTo(ctx, m);
  ctx.globalAlpha = m.alpha;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(scratch, 0, 0, sw, sh, m.mn[0], m.mn[1], sw * cell, sh * cell);
  ctx.restore();
}

function highlight(ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, m: MaskPx) {
  const [w, h] = [m.mx[0] - m.mn[0], m.mx[1] - m.mn[1]];
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, 0, c.width, c.height);
  roundRect(ctx, m.mn[0], m.mn[1], w, h, m.r);
  ctx.fillStyle = `rgba(0, 0, 0, ${(m.dim * m.alpha).toFixed(3)})`;
  ctx.fill("evenodd");
  ctx.restore();
}
