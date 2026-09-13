import type { Cut, Speed, Trim } from "./edit";

/** TS mirror of the Rust `export::remap` module: the SAME algorithms in the SAME expression order
 *  (f64 both sides, no `fround`), so both clocks agree to the bit. The export is the source of truth;
 *  `remap.test.ts` pins the parity table computed from the Rust fixture. Clip time is the raw clip's
 *  clock (0 = the first video frame, every lane's pills); output time is the exported file's. Plain
 *  functions over a frozen value, the codebase's idiom (and what docs-hover resolves). */

export const FACTOR_MIN = 0.25;
export const FACTOR_MAX = 8;

export interface Segment { clipStart: number; clipEnd: number; factor: number; outStart: number }

/** Built once per doc change (`useTimeMap`); every reader below is pure and cheap per tick.
 *  `plain` = no cuts and no speed spans (a trim may exist): every clock function is the identity on
 *  kept time, and the transport hides the clip-time fine print. */
export interface TimeMap { readonly segments: Segment[]; readonly outDur: number; readonly trimIn: number; readonly plain: boolean }

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Mirrors Rust `Trim::resolve`: `out_ms === 0` means the whole clip. */
export function resolveTrim(trim: Trim, fullDurMs: number): [number, number] {
  const out = trim.out_ms === 0 ? fullDurMs : Math.min(trim.out_ms, fullDurMs);
  return [Math.min(trim.in_ms, out), out];
}

function mergedCuts(cuts: Cut[], lo: number, hi: number): [number, number][] {
  const v = cuts.map((c): [number, number] => [clamp(c.start_ms, lo, hi), clamp(c.end_ms, lo, hi)]).filter(([a, b]) => a < b);
  v.sort((x, y) => x[0] - y[0] || x[1] - y[1]);
  const out: [number, number][] = [];
  for (const [a, b] of v) {
    const last = out[out.length - 1];
    if (last && a <= last[1]) last[1] = Math.max(last[1], b); else out.push([a, b]);
  }
  return out;
}

function clampedSpans(speed: Speed[], lo: number, hi: number): [number, number, number][] {
  const v = speed.map((s): [number, number, number] => [clamp(s.start_ms, lo, hi), clamp(s.end_ms, lo, hi), clamp(s.factor, FACTOR_MIN, FACTOR_MAX)]);
  v.sort((x, y) => x[0] - y[0]);
  const out: [number, number, number][] = [];
  for (const [a0, b, f] of v) {
    const last = out[out.length - 1];
    const a = last ? Math.max(a0, last[1]) : a0;
    if (a < b) out.push([a, b, f]);
  }
  return out;
}

/** Trim first, cuts clamped/sorted/merged, speed spans clamped/sorted/kept disjoint, every kept
 *  piece split at span edges and tagged with the span's factor (1 outside). Mirrors `TimeMap::build`. */
export function buildTimeMap(trim: Trim, cuts: Cut[], speed: Speed[], fullDurMs: number): TimeMap {
  const [lo, hi] = resolveTrim(trim, fullDurMs);
  const merged = mergedCuts(cuts, lo, hi);
  const spans = clampedSpans(speed, lo, hi);
  const kept: [number, number][] = [];
  let at = lo;
  for (const [a, b] of merged) { if (at < a) kept.push([at, a]); at = Math.max(at, b); }
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

export const identityMap = (fullDurMs: number): TimeMap => buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], fullDurMs);

/** The exported length, rounded. */
export const outDurMs = (m: TimeMap): number => Math.round(m.outDur);

/** Output time of a clip time; inside a gap, the next segment's start; past the end, the total. */
export function outOf(m: TimeMap, clipMs: number): number {
  for (const s of m.segments) {
    if (clipMs < s.clipStart) return Math.round(s.outStart);
    if (clipMs < s.clipEnd) return Math.round(s.outStart + (clipMs - s.clipStart) / s.factor);
  }
  return Math.round(m.outDur);
}

/** Clip time of an output time on the kept ranges; at or past the end, the last kept edge. */
export function clipOf(m: TimeMap, outMs: number): number {
  for (const s of m.segments) {
    const len = (s.clipEnd - s.clipStart) / s.factor;
    if (outMs < s.outStart + len) return Math.round(s.clipStart + (outMs - s.outStart) * s.factor);
  }
  const last = m.segments[m.segments.length - 1];
  return last ? last.clipEnd : m.trimIn;
}

/** The gap a clip time falls in as `[previous kept end, next kept start]`, the trailing gap running
 *  to `Infinity` (Rust: `u32::MAX`); `null` on a kept range. */
export function gapContaining(m: TimeMap, clipMs: number): [number, number] | null {
  let prevEnd = 0;
  for (const s of m.segments) {
    if (clipMs < s.clipStart) return [prevEnd, s.clipStart];
    if (clipMs < s.clipEnd) return null;
    prevEnd = s.clipEnd;
  }
  return [prevEnd, Infinity];
}

/** The factor of the segment containing a clip time, 1 elsewhere (the preview's `playbackRate`). */
export function factorAt(m: TimeMap, clipMs: number): number {
  const s = m.segments.find((g) => clipMs >= g.clipStart && clipMs < g.clipEnd);
  return s ? s.factor : 1;
}

/** Inclusive recording-frame bounds of segment `i`: the first floors its start, the last floors its
 *  end (today's trim semantics), a cut edge is exact; `null` when empty at this fps. */
export function frameBounds(m: TimeMap, i: number, fps: number): [number, number] | null {
  const s = m.segments[i];
  if (!s) return null;
  const kStart = i === 0 ? Math.floor(s.clipStart * fps / 1000) : Math.floor((s.clipStart * fps + 999) / 1000);
  const kEnd = i + 1 === m.segments.length ? Math.floor(s.clipEnd * fps / 1000) : Math.floor((s.clipEnd * fps + 999) / 1000) - 1;
  return kEnd >= kStart && kEnd >= 0 ? [kStart, kEnd] : null;
}

/** For every output frame, the recording frame it shows. Kept so the parity table pins the plan on
 *  both sides; the preview's media run on clip time and never need it. */
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
