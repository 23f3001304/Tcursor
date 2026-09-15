import type { Cut, Speed, Trim } from "../edit";

export const FACTOR_MIN = 0.25;
export const FACTOR_MAX = 8;

export interface Segment {
  clipStart: number;
  clipEnd: number;
  factor: number;
  outStart: number;
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

export function buildTimeMap(trim: Trim, cuts: Cut[], speed: Speed[], fullDurMs: number): TimeMap {
  const [lo, hi] = resolveTrim(trim, fullDurMs);
  const merged = mergedCuts(cuts, lo, hi);
  const spans = clampedSpans(speed, lo, hi);
  const kept: [number, number][] = [];
  let at = lo;
  for (const [a, b] of merged) {
    if (at < a) kept.push([at, a]);
    at = Math.max(at, b);
  }
  if (at < hi) kept.push([at, hi]);
  const segments: Segment[] = [];
  let outStart = 0;
  for (const [ks, ke] of kept) {
    const edges = [ks, ke];
    for (const [a, b] of spans) for (const e of [a, b]) if (e > ks && e < ke) edges.push(e);
    const uniq = [...new Set(edges)].sort((x, y) => x - y);
    for (let i = 0; i + 1 < uniq.length; i++) {
      const [cs, ce] = [uniq[i], uniq[i + 1]];
      const span = spans.find(([a, b]) => a <= cs && ce <= b);
      const factor = span ? span[2] : 1;
      segments.push({ clipStart: cs, clipEnd: ce, factor, outStart });
      outStart += (ce - cs) / factor;
    }
  }
  return { segments, outDur: outStart, trimIn: lo, plain: merged.length === 0 && spans.length === 0 };
}

export const identityMap = (fullDurMs: number): TimeMap =>
  buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], fullDurMs);

export const outDurMs = (m: TimeMap): number => Math.round(m.outDur);

export function outOf(m: TimeMap, clipMs: number): number {
  for (const s of m.segments) {
    if (clipMs < s.clipStart) return Math.round(s.outStart);
    if (clipMs < s.clipEnd) return Math.round(s.outStart + (clipMs - s.clipStart) / s.factor);
  }
  return Math.round(m.outDur);
}

export function clipOf(m: TimeMap, outMs: number): number {
  for (const s of m.segments) {
    const len = (s.clipEnd - s.clipStart) / s.factor;
    if (outMs < s.outStart + len) return Math.round(s.clipStart + (outMs - s.outStart) * s.factor);
  }
  const last = m.segments[m.segments.length - 1];
  return last ? last.clipEnd : m.trimIn;
}

export function gapContaining(m: TimeMap, clipMs: number): [number, number] | null {
  let prevEnd = 0;
  for (const s of m.segments) {
    if (clipMs < s.clipStart) return [prevEnd, s.clipStart];
    if (clipMs < s.clipEnd) return null;
    prevEnd = s.clipEnd;
  }
  return [prevEnd, Infinity];
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
