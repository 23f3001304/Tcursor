import type { Caption } from "../../../shared/edit";
import type { CaptionStyle } from "../../../hud/settings/settings";

export const ADV = 0.52;

export const LINE_H = 1.32;

export const PAD_X = 0.6;
export const PAD_Y = 0.34;

export const MARGIN = 0.075;

export const RISE_LINES = 0.5;

export const POP_FROM = 0.92;

export const MAX_CHARS = 42;
export const MAX_LINES = 2;

const HEIGHT_FRAC: Record<CaptionStyle["size"], number> = { s: 0.03, m: 0.038, l: 0.048 };

export interface LaidCaption {
  lines: string[];
  fontPx: number;
  lineH: number;
  pill: [number, number, number, number];
  baselines: number[];
  hi: number | null;
  alpha: number;
  rise: number;
  scale: number;
  wordsShown: number | null;
  wordAlpha: number;
}

export function easeOut(p: number): number {
  const q = 1 - Math.min(Math.max(p, 0), 1);
  return 1 - q * q * q;
}

export function ramp(dt: number, ms: number): number {
  return ms <= 0 ? 1 : Math.min(Math.max(dt / ms, 0), 1);
}

export function heightFrac(style: CaptionStyle): number {
  if (style.font_pct > 0) return Math.min(Math.max(style.font_pct / 100, 0.015), 0.08);
  return HEIGHT_FRAC[style.size];
}

export function wrapLines(text: string, max: number): string[] {
  const out: string[] = [];
  for (const w of text.split(/\s+/).filter(Boolean)) {
    const last = out[out.length - 1];
    if (last !== undefined && last.length + 1 + w.length <= max) out[out.length - 1] = `${last} ${w}`;
    else out.push(w);
  }
  return out;
}

export function captionAt(caps: Caption[], tMs: number): Caption | null {
  return caps.find((c) => c.start_ms <= tMs && tMs < c.end_ms) ?? null;
}

export function wordSpan(cap: Caption, i: number): [number, number] | null {
  const w = cap.words[i];
  if (!w) return null;
  let start = 0;
  for (let k = 0; k < i; k++) start += cap.words[k].text.length + 1;
  return [start, start + w.text.length];
}

function reveal(cap: Caption, style: CaptionStyle, tMs: number): [number | null, number] {
  if (style.animation !== "words" || cap.words.length === 0) return [null, 1];
  const n = cap.words.filter((w) => w.start_ms <= tMs).length;
  if (n === 0) return [0, 1];
  return [n, ramp(tMs - cap.words[n - 1].start_ms, style.animation_ms)];
}

export function layoutCaption(
  cap: Caption,
  style: CaptionStyle,
  ow: number,
  oh: number,
  tMs: number,
): LaidCaption {
  const ms = style.animation_ms;
  const anim = style.animation;
  const pIn = ramp(tMs - cap.start_ms, ms);
  // INVARIANT: outside the span every kind is fully transparent, "none" included.
  const live = cap.start_ms <= tMs && tMs < cap.end_ms;
  const alpha = !live ? 0 : anim === "none" ? 1 : Math.min(pIn, ramp(cap.end_ms - tMs, ms));
  const scale = anim === "pop" ? POP_FROM + (1 - POP_FROM) * easeOut(pIn) : 1;
  const fontPx = Math.max(oh * heightFrac(style), 8) * scale;
  const lineH = fontPx * LINE_H;
  const rise = anim === "rise" ? lineH * RISE_LINES * (1 - easeOut(pIn)) : 0;
  let hi: number | null = null;
  if (style.highlight) for (let i = 0; i < cap.words.length; i++) if (cap.words[i].start_ms <= tMs) hi = i;
  const [wordsShown, wordAlpha] = reveal(cap, style, tMs);
  const lines = wrapLines(cap.text.trim(), MAX_CHARS).slice(0, MAX_LINES);
  const zero: [number, number, number, number] = [0, 0, 0, 0];
  const l: LaidCaption = {
    lines,
    fontPx,
    lineH,
    pill: zero,
    baselines: [],
    hi,
    alpha,
    rise,
    scale,
    wordsShown,
    wordAlpha,
  };
  if (lines.length === 0) return l;
  const widest = Math.max(...lines.map((x) => x.length));
  const pw = widest * fontPx * ADV + 2 * fontPx * PAD_X;
  const ph = lines.length * lineH + 2 * fontPx * PAD_Y;
  const px = (ow - pw) / 2;
  const py = rise + (style.position === "bottom" ? oh - oh * MARGIN - ph : oh * MARGIN);
  return {
    ...l,
    pill: [px, py, pw, ph],
    baselines: lines.map((_, i) => py + fontPx * PAD_Y + lineH * i + fontPx),
  };
}
