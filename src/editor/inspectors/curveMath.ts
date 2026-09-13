import { parseCubic } from "../../lib/cubicBezier";
import { spring, springOf } from "../../lib/spring";

export type Cubic = [number, number, number, number];

/** The canvas' own SVG box. x `0..100` is progress `0..1`; y `100..0` is value `0..1`, and the
 *  extra `-30..130` band is the room a handle has to overshoot in either direction. */
export const VIEW = { minX: 0, minY: -30, w: 100, h: 160 } as const;
export const VIEW_BOX = `${VIEW.minX} ${VIEW.minY} ${VIEW.w} ${VIEW.h}`;
const r3 = (v: number) => Math.round(v * 1000) / 1000;
export const Y_MAX = r3(1 - VIEW.minY / 100);
export const Y_MIN = r3(1 - (VIEW.minY + VIEW.h) / 100);

/** The cubic each named curve seeds its handles from when the user starts dragging it. Exact for
 *  linear/ease_in/ease_out/smooth (pinned in curveMath.test.ts against `ease`), the nearest
 *  standard bezier for ease_in_out. A spring is not a cubic at all, so its entry is only the seed
 *  a user gets by dragging the spring's curve into a custom one. The DRAWN paths are not here:
 *  they come from `timeline/curveGlyphs.ts`, the one place the six glyph paths live. */
export const CURVE_SEEDS: Record<string, Cubic> = {
  linear: [1 / 3, 1 / 3, 2 / 3, 2 / 3],
  ease_in: [1 / 3, 0, 2 / 3, 1 / 3],
  ease_out: [1 / 3, 2 / 3, 2 / 3, 1],
  ease_in_out: [0.42, 0, 0.58, 1],
  smooth: [1 / 3, 0, 2 / 3, 1],
  spring: [0.34, 1.25, 0.64, 1],
};

export const toSvg = (x: number, y: number): [number, number] => [x * 100, (1 - y) * 100];

export const clampHandle = (x: number, y: number): [number, number] =>
  [r3(Math.min(1, Math.max(0, x))), r3(Math.min(Y_MAX, Math.max(Y_MIN, y)))];

export function curvePath(c: Cubic): string {
  const [x1, y1] = toSvg(c[0], c[1]);
  const [x2, y2] = toSvg(c[2], c[3]);
  return `M 0 100 C ${r3(x1)} ${r3(y1)}, ${r3(x2)} ${r3(y2)}, 100 0`;
}

export function setHandle(c: Cubic, handle: 0 | 1, x: number, y: number): Cubic {
  const [cx, cy] = clampHandle(x, y);
  const out: Cubic = [...c];
  out[handle * 2] = cx;
  out[handle * 2 + 1] = cy;
  return out;
}

export const nudgeHandle = (c: Cubic, handle: 0 | 1, dx: number, dy: number, step = 0.05): Cubic =>
  setHandle(c, handle, c[handle * 2] + dx * step, c[handle * 2 + 1] + dy * step);

export function clientToCurve(clientX: number, clientY: number,
  rect: { left: number; top: number; width: number; height: number }): [number, number] {
  const u = (clientX - rect.left) / rect.width;
  const v = (clientY - rect.top) / rect.height;
  return clampHandle((VIEW.minX + u * VIEW.w) / 100, 1 - (VIEW.minY + v * VIEW.h) / 100);
}

/** Where a handle sits over the canvas, as `[left%, top%]` of the box. The handles are HTML dots
 *  laid over the SVG rather than circles inside it: the canvas is drawn with
 *  `preserveAspectRatio="none"` (288x96 showing a 100x160 box), which would squash a circle. */
export function handlePct(c: Cubic, handle: 0 | 1): [number, number] {
  const [sx, sy] = toSvg(c[handle * 2], c[handle * 2 + 1]);
  return [r3(((sx - VIEW.minX) / VIEW.w) * 100), r3(((sy - VIEW.minY) / VIEW.h) * 100)];
}

export function curveOf(easing: string): Cubic {
  const parsed = parseCubic(easing);
  if (parsed) return parsed;
  return [...(CURVE_SEEDS[easing] ?? CURVE_SEEDS.smooth)] as Cubic;
}

const SAMPLES = 48;
const r2 = (v: number) => Math.round(v * 100) / 100;

/** The canvas path for an easing that IS a spring, sampled from the same `spring()` the export
 *  evaluates so the drawing cannot lie about the oscillator the sliders are tuning; `null` for
 *  everything else. `curveGlyphs.ts` samples only the ONE default spring for its static glyph,
 *  and keeps that sampler private, so the live one is computed here. */
export function springPathOf(easing: string | undefined | null): string | null {
  const s = springOf(easing);
  if (!s) return null;
  const pts: string[] = [];
  for (let i = 1; i <= SAMPLES; i++) {
    const p = i / SAMPLES;
    pts.push(`${r2(p * 100)} ${r2((1 - spring(s[0], s[1], s[2], p)) * 100)}`);
  }
  return `M 0 100 L ${pts.join(" L ")}`;
}
