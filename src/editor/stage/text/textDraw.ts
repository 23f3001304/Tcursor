import { roundRect } from "../canvas/previewDraw";
import type { LaidText } from "./textPreview";

const PLATE_RADIUS = 0.35;
const SUB_ALPHA = 0.82;
const PAD_X = 0.6;

const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

const faceAt = (px: number) => `600 ${px}px "Inter Variable", system-ui, sans-serif`;

export function drawTexts(ctx: CanvasRenderingContext2D, laid: LaidText[]) {
  for (const l of laid) {
    if (l.alpha <= 0) continue;
    ctx.save();
    if (l.plate[2] > 0 && l.plate[3] > 0) {
      ctx.globalAlpha = l.plateAlpha * l.alpha;
      ctx.fillStyle = rgb(l.plateRgb);
      roundRect(ctx, l.plate[0], l.plate[1], l.plate[2], l.plate[3], l.fontPx * PLATE_RADIUS);
      ctx.fill();
    }
    if (l.rule[2] > 0 && l.rule[3] > 0) {
      ctx.globalAlpha = l.alpha;
      ctx.fillStyle = rgb(l.ruleRgb);
      ctx.fillRect(l.rule[0], l.rule[1], l.rule[2], l.rule[3]);
    }
    ctx.textBaseline = "alphabetic";
    ctx.textAlign = "left";
    ctx.fillStyle = rgb(l.fill);
    if (l.shadow) {
      ctx.shadowColor = `rgba(0, 0, 0, ${(0.6 * l.alpha).toFixed(3)})`;
      ctx.shadowOffsetX = 1;
      ctx.shadowOffsetY = 1;
    }
    const shown = l.reveal === null ? l.main : [...l.main].slice(0, l.reveal).join("");
    if (shown) {
      ctx.globalAlpha = l.alpha;
      ctx.font = faceAt(l.fontPx);
      ctx.fillText(shown, runX(ctx, l, shown, l.fontPx), l.mainBaseline[1]);
    }
    if (l.sub) {
      ctx.globalAlpha = l.alpha * SUB_ALPHA;
      ctx.font = faceAt(l.subPx);
      ctx.fillText(l.sub, runX(ctx, l, l.sub, l.subPx), l.subBaseline[1]);
    }
    ctx.restore();
  }
}

function runX(ctx: CanvasRenderingContext2D, l: LaidText, text: string, px: number): number {
  if (!l.centred) return l.mainBaseline[0];
  const inner = l.plate[2] > 0 ? l.plate[2] - 2 * l.fontPx * PAD_X : l.boxW;
  const save = ctx.font;
  ctx.font = faceAt(px);
  const w = ctx.measureText(text).width;
  ctx.font = save;
  return l.mainBaseline[0] + (inner - w) / 2;
}
