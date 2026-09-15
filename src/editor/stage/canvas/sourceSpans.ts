import type { SourceSpanDto } from "../../../shared/ipc";

export const FULL_SRC: [number, number, number, number] = [0, 0, 1, 1];

export interface SpanState {
  src: [number, number, number, number];
  fit: [number, number];
}

export const lerpN = (a: number, b: number, t: number) => a + (b - a) * t;

export function activeSpanIdx(spans: SourceSpanDto[], t: number): number {
  let idx = -1;
  for (let k = 0; k < spans.length; k++) if (t >= spans[k].start_ms) idx = k;
  return idx;
}

export function spanAt(
  spans: SourceSpanDto[],
  t: number,
  ease: (name: string, p: number) => number,
): SpanState {
  const i = activeSpanIdx(spans, t);
  if (i < 0) return { src: FULL_SRC, fit: [1, 1] };
  const s = spans[i];
  if (i === 0 || s.transition_ms <= 0) return { src: s.src, fit: s.fit };
  const elapsed = t - s.start_ms;
  if (elapsed >= s.transition_ms) return { src: s.src, fit: s.fit };
  const prev = spans[i - 1];
  const f = ease("smooth", elapsed / s.transition_ms);
  return { src: s.src, fit: [lerpN(prev.fit[0], s.fit[0], f), lerpN(prev.fit[1], s.fit[1], f)] };
}

export function fitPanel(
  rect: [number, number, number, number],
  fit: [number, number],
): [number, number, number, number] {
  const [x, y, w, h] = rect;
  const [fw, fh] = [w * fit[0], h * fit[1]];
  return [x + (w - fw) / 2, y + (h - fh) / 2, fw, fh];
}

export function toPanelFrac(x: number, y: number, src: [number, number, number, number]): [number, number] {
  return [(x - src[0]) / Math.max(src[2], 1e-6), (y - src[1]) / Math.max(src[3], 1e-6)];
}
