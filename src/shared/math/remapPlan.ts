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
