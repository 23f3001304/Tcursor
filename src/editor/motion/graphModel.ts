import { ease } from "../timeline/model/layoutTrack";
import { parseKeys, toKeys, type KeyMode } from "./keys";
import { graphKeys, r2, sampleRamp, SAMPLES_PER_RAMP } from "./graphCoords";

export type Lane = "zoom" | "layout" | "cam";
export interface RampSpec {
  easing: string;
  durMs: number;
}

export interface GraphInput {
  lane: Lane;
  startMs: number;
  endMs: number;
  rampIn: RampSpec;
  rampOut: RampSpec | null;
  peak: number;
  followHint?: boolean;
  prev?: { endMs: number; easingOut: string; durMs: number } | null;
  next?: { startMs: number; easingIn: string; durMs: number } | null;
}

export interface GraphKey {
  i: number;
  x: number;
  y: number;
  inX: number;
  inY: number;
  outX: number;
  outY: number;
  mode: KeyMode;
}
export interface GraphRamp {
  which: "in" | "out";
  x0: number;
  x1: number;
  path: string;
  keys: GraphKey[] | null;
  editable: boolean;
}
export interface GraphBand {
  x0: number;
  x1: number;
  y: number;
}
export interface GraphModel {
  width: number;
  height: number;
  plot: { x: number; y: number; w: number; h: number };
  ramps: GraphRamp[];
  plateau: GraphBand | null;
  hint: GraphBand | null;
  ghosts: string[];
  ticks: { x: number; label: string }[];
  yTicks: { y: number; label: string }[];
}

export const GRAPH_W = 320;
export const GRAPH_H = 140;

export const GHOST_PX = 16;
const GHOST_SAMPLES = 16;
const PAD = { l: 26, r: 6, t: 12, b: 18 };

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

export const baseOf = (lane: Lane) => (lane === "zoom" ? 1 : 0);

const STEPS = [50, 100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 20000];

export function niceStep(spanMs: number): number {
  return STEPS.find((s) => spanMs / s <= 5) ?? Math.ceil(spanMs / 5);
}
const tickLabel = (ms: number) => (ms < 1000 ? `${Math.round(ms)}ms` : `${(ms / 1000).toFixed(1)}s`);

export function buildGraph(input: GraphInput, width = GRAPH_W, height = GRAPH_H): GraphModel {
  const plot = {
    x: PAD.l + GHOST_PX,
    y: PAD.t,
    w: Math.max(1, width - PAD.l - PAD.r - GHOST_PX * 2),
    h: Math.max(1, height - PAD.t - PAD.b),
  };
  const base = baseOf(input.lane),
    peak = input.peak;
  const span = Math.max(1, input.endMs - input.startMs);
  const mx = (ms: number) => plot.x + ((ms - input.startMs) / span) * plot.w;
  const xMs = (x: number) => input.startMs + ((x - plot.x) / plot.w) * span;

  const specs: { which: "in" | "out"; spec: RampSpec }[] = [{ which: "in", spec: input.rampIn }];
  if (input.rampOut) specs.push({ which: "out", spec: input.rampOut });
  const parsed = specs.map((s) => parseKeys(s.spec.easing));
  const values = specs.map((s, i) => sampleRamp(s.spec, parsed[i], base, peak, s.which));

  let lo = Math.min(base, peak),
    hi = Math.max(base, peak);
  for (const vs of values)
    for (const v of vs) {
      lo = Math.min(lo, v);
      hi = Math.max(hi, v);
    }
  const pad = (hi - lo) * 0.08 || 0.08;
  lo -= pad;
  hi += pad;
  const yPx = (v: number) => plot.y + plot.h * (1 - (v - lo) / Math.max(1e-6, hi - lo));

  const inEnd = input.startMs + Math.max(0, input.rampIn.durMs);
  const outStart = input.endMs - Math.max(0, input.rampOut?.durMs ?? 0);
  const ramps: GraphRamp[] = specs.map((s, i) => {
    const x0 = s.which === "in" ? mx(input.startMs) : mx(outStart);
    const x1 = s.which === "in" ? mx(inEnd) : mx(input.endMs);
    const pts = values[i].map((v, n) => `${r2(x0 + (n / SAMPLES_PER_RAMP) * (x1 - x0))} ${r2(yPx(v))}`);
    return {
      which: s.which,
      x0: r2(x0),
      x1: r2(x1),
      path: `M ${pts.join(" L ")}`,
      keys: parsed[i] ? graphKeys(parsed[i]!, x0, x1, s.which, base, peak, yPx) : null,
      editable: parsed[i] !== null || toKeys(s.spec.easing) !== null,
    };
  });

  const plateau: GraphBand | null =
    outStart > inEnd ? { x0: r2(mx(inEnd)), x1: r2(mx(outStart)), y: r2(yPx(peak)) } : null;

  const ghost = (
    fromMs: number,
    toMs: number,
    easing: string,
    falling: boolean,
    gx0: number,
    gx1: number,
  ) => {
    const dur = Math.max(1, toMs - fromMs);
    const pts: string[] = [];
    for (let i = 0; i <= GHOST_SAMPLES; i++) {
      const x = gx0 + (i / GHOST_SAMPLES) * (gx1 - gx0);
      const e = ease(easing, clamp01((xMs(x) - fromMs) / dur));
      pts.push(`${r2(x)} ${r2(yPx(base + (peak - base) * (falling ? 1 - e : e)))}`);
    }
    return `M ${pts.join(" L ")}`;
  };
  const ghosts: string[] = [];
  if (input.prev)
    ghosts.push(
      ghost(
        input.prev.endMs - input.prev.durMs,
        input.prev.endMs,
        input.prev.easingOut,
        true,
        plot.x - GHOST_PX,
        plot.x,
      ),
    );
  if (input.next)
    ghosts.push(
      ghost(
        input.next.startMs,
        input.next.startMs + input.next.durMs,
        input.next.easingIn,
        false,
        plot.x + plot.w,
        plot.x + plot.w + GHOST_PX,
      ),
    );

  const step = niceStep(span);
  const ticks: { x: number; label: string }[] = [];
  for (let ms = 0; ms <= span; ms += step)
    ticks.push({ x: r2(mx(input.startMs + ms)), label: tickLabel(ms) });
  const yLabel = (v: number) => (input.lane === "zoom" ? `${v.toFixed(1)}x` : `${Math.round(v * 100)}%`);

  return {
    width,
    height,
    plot,
    ramps,
    plateau,
    hint: input.followHint && plateau ? { ...plateau } : null,
    ghosts,
    ticks,
    yTicks: [
      { y: r2(yPx(base)), label: yLabel(base) },
      { y: r2(yPx(peak)), label: yLabel(peak) },
    ],
  };
}
