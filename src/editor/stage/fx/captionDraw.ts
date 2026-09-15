import type { Caption } from "../../../shared/edit";
import type { CaptionStyle } from "../../../hud/settings/settings";
import { captionAt, layoutCaption, wordSpan } from "./captionPreview";

const RADIUS = 0.3;

const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

const cutAt = (cap: Caption, shown: number): [number, number] =>
  (shown > 0 ? wordSpan(cap, shown - 1) : null) ?? [0, 0];

interface Pen {
  alpha: number;
  text: string;
  lit: string;
  hi: [number, number] | null;
  cut: [number, number] | null;
  wordAlpha: number;
}

export function drawCaptions(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  caps: Caption[],
  style: CaptionStyle,
  accent: [number, number, number],
  t: number,
) {
  if (!style.enabled) return;
  const cap = captionAt(caps, t);
  if (!cap) return;
  const l = layoutCaption(cap, style, w, h, t);
  if (l.alpha <= 0 || l.lines.length === 0) return;
  const [px, py, pw, ph] = l.pill;
  ctx.save();
  if (style.pill) {
    const [pr, pg, pb] = style.pill_color;
    ctx.fillStyle = `rgba(${pr}, ${pg}, ${pb}, ${(style.pill_alpha / 100) * l.alpha})`;
    ctx.beginPath();
    ctx.roundRect(px, py, pw, ph, Math.min(ph * RADIUS, pw / 2));
    ctx.fill();
  }
  ctx.font = `600 ${l.fontPx}px "Inter Variable", system-ui, sans-serif`;
  ctx.textBaseline = "alphabetic";
  ctx.shadowColor = `rgba(0, 0, 0, ${0.6 * l.alpha})`;
  ctx.shadowOffsetX = 1;
  ctx.shadowOffsetY = 1;
  const pen: Pen = {
    alpha: l.alpha,
    text: rgb(style.text_color),
    lit: rgb(style.highlight_color ?? accent),
    hi: l.hi === null ? null : wordSpan(cap, l.hi),
    cut: l.wordsShown === null ? null : cutAt(cap, l.wordsShown),
    wordAlpha: l.wordAlpha,
  };
  const cx = px + pw / 2;
  let base = 0;
  for (let i = 0; i < l.lines.length; i++) {
    drawLine(ctx, l.lines[i], cx, l.baselines[i], pen, base);
    base += l.lines[i].length + 1;
  }
  ctx.restore();
}

function drawLine(
  ctx: CanvasRenderingContext2D,
  line: string,
  cx: number,
  baseline: number,
  pen: Pen,
  base: number,
) {
  const left = cx - ctx.measureText(line).width / 2;
  const at = (i: number) => left + ctx.measureText(line.slice(0, i)).width;
  const run = (s: number, e: number, color: string, alpha: number) => {
    const [from, to] = [Math.max(s, 0), Math.min(e, line.length)];
    if (to <= from || alpha <= 0) return;
    ctx.globalAlpha = alpha;
    ctx.fillStyle = color;
    ctx.fillText(line.slice(from, to), at(from), baseline);
  };
  ctx.textAlign = "left";
  if (pen.cut === null) run(0, line.length, pen.text, pen.alpha);
  else {
    // INVARIANT: an unrevealed word keeps its place, so the run never reflows.
    run(0, pen.cut[0] - base, pen.text, pen.alpha);
    run(pen.cut[0] - base, pen.cut[1] - base, pen.text, pen.alpha * pen.wordAlpha);
  }
  if (pen.hi) {
    const s = pen.hi[0] - base;
    const e = pen.cut ? Math.min(pen.hi[1] - base, pen.cut[1] - base) : pen.hi[1] - base;
    run(s, e, pen.lit, pen.cut && s >= pen.cut[0] - base ? pen.alpha * pen.wordAlpha : pen.alpha);
  }
  ctx.textAlign = "center";
}
