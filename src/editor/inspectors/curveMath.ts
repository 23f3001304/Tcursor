import { parseCubic } from "../../lib/cubicBezier";
import { CAM_CURVES } from "./curves";

export type Cubic = [number, number, number, number];

/** The curve cards' shared SVG viewBox. x `0..100` is progress `0..1`; y `100..0` is value `0..1`,
 *  and the extra `-30..130` band is the room a handle has to overshoot in either direction. */
export const VIEW = { minX: 0, minY: -30, w: 100, h: 160 } as const;
export const VIEW_BOX = `${VIEW.minX} ${VIEW.minY} ${VIEW.w} ${VIEW.h}`;
const r3 = (v: number) => Math.round(v * 1000) / 1000;
/** Value bounds implied by the viewBox band - the drag/nudge clamp. Rounded like every other
 *  coordinate this module emits, so a clamped handle compares equal to the bound itself. */
export const Y_MAX = r3(1 - VIEW.minY / 100);
export const Y_MIN = r3(1 - (VIEW.minY + VIEW.h) / 100);

/** Curve space (progress, value) -> SVG user units. */
export const toSvg = (x: number, y: number): [number, number] => [x * 100, (1 - y) * 100];

/** Clamp a control point to what the editor can show and the Rust parser accepts: x into `[0,1]`
 *  (keeps `x(t)` monotonic), y into the viewBox's overshoot band. */
export const clampHandle = (x: number, y: number): [number, number] =>
  [r3(Math.min(1, Math.max(0, x))), r3(Math.min(Y_MAX, Math.max(Y_MIN, y)))];

/** The `d` for a cubic through P0=(0,0), P1, P2, P3=(1,1). */
export function curvePath(c: Cubic): string {
  const [x1, y1] = toSvg(c[0], c[1]);
  const [x2, y2] = toSvg(c[2], c[3]);
  return `M 0 100 C ${r3(x1)} ${r3(y1)}, ${r3(x2)} ${r3(y2)}, 100 0`;
}

/** Replace one control point (0 = P1, 1 = P2), clamped. */
export function setHandle(c: Cubic, handle: 0 | 1, x: number, y: number): Cubic {
  const [cx, cy] = clampHandle(x, y);
  const out: Cubic = [...c];
  out[handle * 2] = cx;
  out[handle * 2 + 1] = cy;
  return out;
}

/** Keyboard nudge: one arrow press moves a handle by `step` in curve space. */
export const nudgeHandle = (c: Cubic, handle: 0 | 1, dx: number, dy: number, step = 0.05): Cubic =>
  setHandle(c, handle, c[handle * 2] + dx * step, c[handle * 2 + 1] + dy * step);

/** Pointer position -> curve space, clamped. `rect` is the editor SVG's bounding box. */
export function clientToCurve(clientX: number, clientY: number,
  rect: { left: number; top: number; width: number; height: number }): [number, number] {
  const u = (clientX - rect.left) / rect.width;
  const v = (clientY - rect.top) / rect.height;
  return clampHandle((VIEW.minX + u * VIEW.w) / 100, 1 - (VIEW.minY + v * VIEW.h) / 100);
}

/** The control points the editor should show for an easing wire-name: the parsed curve for a
 *  custom `cubic(...)`, else the named preset's editing seed, else Smooth's (what `valid_easing`
 *  coerces an unknown name to anyway). */
export function curveOf(easing: string): Cubic {
  const parsed = parseCubic(easing);
  if (parsed) return parsed;
  const named = CAM_CURVES.find((c) => c.key === easing) ?? CAM_CURVES.find((c) => c.key === "smooth");
  return [...(named?.c ?? [1 / 3, 0, 2 / 3, 1])] as Cubic;
}
