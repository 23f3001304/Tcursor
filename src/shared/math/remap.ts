import type { Clip, Cut, Speed, Trim } from "../edit";

export const FACTOR_MIN = 0.25;
export const FACTOR_MAX = 8;

export interface Segment {
  clipStart: number;
  clipEnd: number;
  factor: number;
  outStart: number;
  clip: number;
}

export interface TimeMap {
  readonly segments: Segment[];
  readonly outDur: number;
  readonly trimIn: number;
  readonly plain: boolean;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function resolveTrim(trim: Trim, fullDurMs: number): [number, number] {
  const out = trim.out_ms === 0 ? fullDurMs : Math.min(trim.out_ms, fullDurMs);
  return [Math.min(trim.in_ms, out), out];
}

function mergedCuts(cuts: Cut[], lo: number, hi: number): [number, number][] {
  const v = cuts
    .map((c): [number, number] => [clamp(c.start_ms, lo, hi), clamp(c.end_ms, lo, hi)])
    .filter(([a, b]) => a < b);
  v.sort((x, y) => x[0] - y[0] || x[1] - y[1]);
  const out: [number, number][] = [];
  for (const [a, b] of v) {
    const last = out[out.length - 1];
    if (last && a <= last[1]) last[1] = Math.max(last[1], b);
    else out.push([a, b]);
  }
  return out;
}

function clampedSpans(speed: Speed[], lo: number, hi: number): [number, number, number][] {
  const v = speed.map((s): [number, number, number] => [
    clamp(s.start_ms, lo, hi),
    clamp(s.end_ms, lo, hi),
    clamp(s.factor, FACTOR_MIN, FACTOR_MAX),
  ]);
  v.sort((x, y) => x[0] - y[0]);
  const out: [number, number, number][] = [];
  for (const [a0, b, f] of v) {
    const last = out[out.length - 1];
    const a = last ? Math.max(a0, last[1]) : a0;
    if (a < b) out.push([a, b, f]);
  }
  return out;
}

function clipRanges(trim: Trim, clips: Clip[], fullDurMs: number): [number, number, number][] {
  if (clips.length === 0) {
    const [lo, hi] = resolveTrim(trim, fullDurMs);
    return [[0, lo, hi]];
  }
  return clips
    .map((c, i): [number, number, number] => [
      i,
      Math.min(c.src_in_ms, fullDurMs),
      Math.min(c.src_out_ms, fullDurMs),
    ])
    .filter(([, a, b]) => a < b);
}

export function buildTimeMap(
  trim: Trim,
  cuts: Cut[],
  speed: Speed[],
  clips: Clip[],
  fullDurMs: number,
): TimeMap {
  const [lo, hi] = resolveTrim(trim, fullDurMs);
  const ranges = clipRanges(trim, clips, fullDurMs);
  const segments: Segment[] = [];
  let outStart = 0;
  let plain = ranges.length === 1 && ranges[0][1] === lo && ranges[0][2] === hi;
  for (const [ci, rlo, rhi] of ranges) {
    const merged = mergedCuts(cuts, rlo, rhi);
    const spans = clampedSpans(speed, rlo, rhi);
    plain = plain && merged.length === 0 && spans.length === 0;
    const kept: [number, number][] = [];
    let at = rlo;
    for (const [a, b] of merged) {
      if (at < a) kept.push([at, a]);
      at = Math.max(at, b);
    }
    if (at < rhi) kept.push([at, rhi]);
    for (const [ks, ke] of kept) {
      const edges = [ks, ke];
      for (const [a, b] of spans) for (const e of [a, b]) if (e > ks && e < ke) edges.push(e);
      const uniq = [...new Set(edges)].sort((x, y) => x - y);
      for (let i = 0; i + 1 < uniq.length; i++) {
        const [cs, ce] = [uniq[i], uniq[i + 1]];
        const span = spans.find(([a, b]) => a <= cs && ce <= b);
        const factor = span ? span[2] : 1;
        segments.push({ clipStart: cs, clipEnd: ce, factor, outStart, clip: ci });
        outStart += (ce - cs) / factor;
      }
    }
  }
  return { segments, outDur: outStart, trimIn: lo, plain };
}

export const identityMap = (fullDurMs: number): TimeMap =>
  buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], [], fullDurMs);

export const outDurMs = (m: TimeMap): number => Math.round(m.outDur);

export function nextShown(m: TimeMap, clipMs: number): Segment | undefined {
  let next: Segment | undefined;
  for (const g of m.segments)
    if (
      g.clipStart > clipMs &&
      (!next ||
        g.clipStart < next.clipStart ||
        (g.clipStart === next.clipStart && g.outStart < next.outStart))
    )
      next = g;
  return next;
}

export function outOf(m: TimeMap, clipMs: number): number {
  const s = m.segments.find((g) => g.clipStart <= clipMs && clipMs < g.clipEnd);
  if (s) return Math.round(s.outStart + (clipMs - s.clipStart) / s.factor);
  const next = nextShown(m, clipMs);
  return Math.round(next ? next.outStart : m.outDur);
}

export function clipOf(m: TimeMap, outMs: number): number {
  for (const s of m.segments) {
    const len = (s.clipEnd - s.clipStart) / s.factor;
    if (outMs < s.outStart + len) return Math.round(s.clipStart + (outMs - s.outStart) * s.factor);
  }
  const last = m.segments[m.segments.length - 1];
  return last ? last.clipEnd : m.trimIn;
}

function segOfOut(m: TimeMap, outMs: number): number {
  return m.segments.findIndex((s) => outMs < s.outStart + (s.clipEnd - s.clipStart) / s.factor);
}

export function crossesBoundary(m: TimeMap, prevOutMs: number, outMs: number): boolean {
  const [a, b] = [segOfOut(m, prevOutMs), segOfOut(m, outMs)];
  if (a < 0 || b < 0 || a >= b) return false;
  for (let i = a; i < b; i++) if (m.segments[i].clipEnd !== m.segments[i + 1].clipStart) return true;
  return false;
}

export function clipOutMs(m: TimeMap, clip: number): number {
  return Math.round(
    m.segments
      .filter((s) => s.clip === clip)
      .reduce((acc, s) => acc + (s.clipEnd - s.clipStart) / s.factor, 0),
  );
}

export function factorAt(m: TimeMap, clipMs: number): number {
  const s = m.segments.find((g) => clipMs >= g.clipStart && clipMs < g.clipEnd);
  return s ? s.factor : 1;
}

export function frameBounds(m: TimeMap, i: number, fps: number): [number, number] | null {
  const s = m.segments[i];
  if (!s) return null;
  const kStart =
    i === 0 ? Math.floor((s.clipStart * fps) / 1000) : Math.floor((s.clipStart * fps + 999) / 1000);
  const kEnd =
    i + 1 === m.segments.length
      ? Math.floor((s.clipEnd * fps) / 1000)
      : Math.floor((s.clipEnd * fps + 999) / 1000) - 1;
  return kEnd >= kStart && kEnd >= 0 ? [kStart, kEnd] : null;
}

export function framePlan(m: TimeMap, fps: number): number[] {
  const plan: number[] = [];
  m.segments.forEach((s, i) => {
    const b = frameBounds(m, i, fps);
    if (!b) return;
    const n = Math.floor((b[1] - b[0]) / s.factor) + 1;
    for (let j = 0; j < n; j++) plan.push(b[0] + Math.floor(j * s.factor));
  });
  return plan;
}
