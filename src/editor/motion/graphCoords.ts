import { ease } from "../timeline/model/layoutTrack";
import { evalKeys, type Keys } from "./keys";
import type { GraphInput, GraphKey, GraphModel, RampSpec } from "./graphModel";

export const SAMPLES_PER_RAMP = 64;

export const r2 = (v: number) => Math.round(v * 100) / 100;

export function sampleRamp(
  spec: RampSpec,
  k: Keys | null,
  base: number,
  peak: number,
  which: "in" | "out",
): number[] {
  const out: number[] = [];
  for (let i = 0; i <= SAMPLES_PER_RAMP; i++) {
    const p = i / SAMPLES_PER_RAMP;
    const e = k ? evalKeys(k, p) : ease(spec.easing, p);
    out.push(base + (peak - base) * (which === "in" ? e : 1 - e));
  }
  return out;
}

export function graphKeys(
  k: Keys,
  x0: number,
  x1: number,
  which: "in" | "out",
  base: number,
  peak: number,
  yPx: (v: number) => number,
): GraphKey[] {
  const px = (t: number) => x0 + t * (x1 - x0);
  const py = (v: number) => yPx(base + (peak - base) * (which === "in" ? v : 1 - v));
  return k.keys.map((key, i) => ({
    i,
    x: r2(px(key.t)),
    y: r2(py(key.v)),
    inX: r2(px(key.t + key.in[0])),
    inY: r2(py(key.v + key.in[1])),
    outX: r2(px(key.t + key.out[0])),
    outY: r2(py(key.v + key.out[1])),
    mode: key.mode,
  }));
}

export const msToX = (m: GraphModel, input: GraphInput, ms: number): number =>
  m.plot.x + ((ms - input.startMs) / Math.max(1, input.endMs - input.startMs)) * m.plot.w;

export const xToMs = (m: GraphModel, input: GraphInput, x: number): number =>
  input.startMs + ((x - m.plot.x) / Math.max(1e-6, m.plot.w)) * Math.max(1, input.endMs - input.startMs);

export const progressToY = (m: GraphModel, which: "in" | "out", v: number): number => {
  const p = which === "in" ? v : 1 - v;
  return m.yTicks[0].y + p * (m.yTicks[1].y - m.yTicks[0].y);
};

export const yToProgress = (m: GraphModel, which: "in" | "out", y: number): number => {
  const d = m.yTicks[1].y - m.yTicks[0].y;
  const p = (y - m.yTicks[0].y) / (Math.abs(d) < 1e-6 ? 1 : d);
  return which === "in" ? p : 1 - p;
};

export function clientToPlot(
  m: GraphModel,
  rect: { left: number; top: number; width: number; height: number },
  clientX: number,
  clientY: number,
): [number, number] {
  const sx = m.width / Math.max(1, rect.width),
    sy = m.height / Math.max(1, rect.height);
  return [(clientX - rect.left) * sx, (clientY - rect.top) * sy];
}
