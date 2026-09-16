import type { EffectRegion } from "../../../shared/edit";
import type { PreviewLayout } from "../../../shared/ipc";
import { fxFrameGeometry } from "../fx/fxGeometry";

export const DEFAULT_BLUR = 0.02;
export const DEFAULT_PIXEL = 0.018;
export const DEFAULT_MASK_ROUNDNESS = 0.06;
export const DEFAULT_MASK_FEATHER = 0.01;
export const HIDDEN_PANEL_ALPHA = 0.5;

export type MaskKindId = 1 | 2 | 3;

export interface MaskPx {
  mn: [number, number];
  mx: [number, number];
  r: number;
  featherPx: number;
  amountPx: number;
  dim: number;
  kind: MaskKindId;
  alpha: number;
}

const KIND_ID: Record<string, MaskKindId> = { blur: 1, pixelate: 2, highlight: 3 };

export function blurSigmaFor(amountPx: number): number {
  const r = Math.max(1, Math.round(amountPx));
  const k = 2 * r + 1;
  return Math.sqrt((k * k - 1) / 4);
}

function regionAlpha(e: EffectRegion, ms: number): number {
  if (ms < e.start_ms || ms >= e.end_ms) return 0;
  const inn = (ms - e.start_ms) / Math.max(1, e.fade_in_ms);
  const outn = (e.end_ms - ms) / Math.max(1, e.fade_out_ms);
  return Math.min(1, Math.max(0, Math.min(inn, outn)));
}

export function maskDraws(
  effects: EffectRegion[],
  layout: PreviewLayout | null,
  cam: { cx: number; cy: number; scale: number },
  w: number,
  h: number,
  tMs: number,
  defaultDim: number,
): MaskPx[] {
  if ((layout?.screenAlpha ?? 1) < HIDDEN_PANEL_ALPHA) return [];
  const g = fxFrameGeometry(w, h, layout, cam, 1);
  return effects
    .filter((e) => KIND_ID[e.kind] !== undefined && e.rect)
    .map((e, i) => ({ e, key: [e.layer ?? 0, i] as [number, number] }))
    .sort((a, b) => a.key[0] - b.key[0] || a.key[1] - b.key[1])
    .map(({ e }) => one(e, g, h, tMs, defaultDim))
    .filter((m): m is MaskPx => m !== null);
}

function one(
  e: EffectRegion,
  g: ReturnType<typeof fxFrameGeometry>,
  h: number,
  tMs: number,
  defaultDim: number,
): MaskPx | null {
  const alpha = regionAlpha(e, tMs);
  const rect = e.rect;
  if (alpha <= 0 || !rect) return null;
  const box = projectRect(rect, g);
  if (!box) return null;
  const [mn, mx] = box;
  const short = Math.min(mx[0] - mn[0], mx[1] - mn[1]);
  const kind = KIND_ID[e.kind];
  const amount = e.strength ?? (kind === 2 ? DEFAULT_PIXEL : DEFAULT_BLUR);
  return {
    mn,
    mx,
    r: short * Math.min(0.5, Math.max(0, e.roundness ?? DEFAULT_MASK_ROUNDNESS)),
    featherPx: h * Math.max(0, e.feather ?? DEFAULT_MASK_FEATHER),
    amountPx: h * Math.max(0, amount),
    dim: Math.min(1, Math.max(0, e.dim ?? defaultDim)),
    kind,
    alpha,
  };
}

function projectRect(
  rect: [number, number, number, number],
  g: ReturnType<typeof fxFrameGeometry>,
): [[number, number], [number, number]] | null {
  const corners: [number, number][] = [
    [rect[0], rect[1]],
    [rect[0] + rect[2], rect[1]],
    [rect[0], rect[1] + rect[3]],
    [rect[0] + rect[2], rect[1] + rect[3]],
  ];
  let mn: [number, number] = [Infinity, Infinity];
  let mx: [number, number] = [-Infinity, -Infinity];
  for (const [fx, fy] of corners) {
    const p = g.mapCanvas(fx, fy);
    if (!p) return null;
    mn = [Math.min(mn[0], p[0]), Math.min(mn[1], p[1])];
    mx = [Math.max(mx[0], p[0]), Math.max(mx[1], p[1])];
  }
  const lo = g.map(0, 0);
  const hi = g.map(1, 1);
  if (!lo || !hi) return null;
  mn = [Math.max(mn[0], lo[0]), Math.max(mn[1], lo[1])];
  mx = [Math.min(mx[0], hi[0]), Math.min(mx[1], hi[1])];
  return mx[0] - mn[0] >= 1 && mx[1] - mn[1] >= 1 ? [mn, mx] : null;
}
