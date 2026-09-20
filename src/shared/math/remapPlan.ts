import { frameBounds, type TimeMap } from "./remap";

export function planBoundaries(m: TimeMap, fps: number): number[] {
  const out: number[] = [];
  let lastEnd: number | undefined;
  let at = 0;
  m.segments.forEach((s, i) => {
    const b = frameBounds(m, i, fps);
    if (!b) return;
    if (lastEnd !== undefined && lastEnd !== s.clipStart) out.push(at);
    lastEnd = s.clipEnd;
    at += Math.floor((b[1] - b[0]) / s.factor) + 1;
  });
  return out;
}

export interface ClipSpan {
  clip: number;
  planStart: number;
  planLen: number;
  firstK: number;
}

export function clipSpans(m: TimeMap, fps: number): ClipSpan[] {
  const out: ClipSpan[] = [];
  let at = 0;
  m.segments.forEach((s, i) => {
    const b = frameBounds(m, i, fps);
    if (!b) return;
    const n = Math.floor((b[1] - b[0]) / s.factor) + 1;
    const last = out[out.length - 1];
    if (last && last.clip === s.clip) last.planLen += n;
    else out.push({ clip: s.clip, planStart: at, planLen: n, firstK: b[0] });
    at += n;
  });
  return out;
}
