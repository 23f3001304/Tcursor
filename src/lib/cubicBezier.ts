/** TS mirror of the Rust `export::cubic` module - the SAME math, because the export is the source
 *  of truth and this only drives the live preview. Every change here needs the matching change
 *  there (and vice versa); `cubicBezier.test.ts` pins the shared numbers on this side. */

const bez = (a: number, b: number, t: number) => {
  const c1 = 3 * a, c2 = 3 * b - 6 * a, c3 = 3 * a - 3 * b + 1;
  return ((c3 * t + c2) * t + c1) * t;
};
const dbez = (a: number, b: number, t: number) => {
  const c1 = 3 * a, c2 = 3 * b - 6 * a, c3 = 3 * a - 3 * b + 1;
  return (3 * c3 * t + 2 * c2) * t + c1;
};

/** The curve parameter where `x(t) === x`: Newton first, bisection as the guaranteed fallback. */
function solveT(x1: number, x2: number, x: number): number {
  let t = x;
  for (let i = 0; i < 8; i++) {
    const e = bez(x1, x2, t) - x;
    if (Math.abs(e) < 1e-7) return t;
    const d = dbez(x1, x2, t);
    if (Math.abs(d) < 1e-6) break;
    t = Math.min(1, Math.max(0, t - e / d));
  }
  let lo = 0, hi = 1;
  t = x;
  for (let i = 0; i < 30; i++) {
    const e = bez(x1, x2, t) - x;
    if (Math.abs(e) < 1e-7) return t;
    if (e > 0) hi = t; else lo = t;
    t = (lo + hi) / 2;
  }
  return t;
}

/** Evaluate a CSS-semantics cubic-bezier easing at progress `p` (clamped). Endpoints are exact. */
export function evalCubic(x1: number, y1: number, x2: number, y2: number, p: number): number {
  const c = Math.min(1, Math.max(0, p));
  if (c <= 0) return 0;
  if (c >= 1) return 1;
  return bez(y1, y2, solveT(x1, x2, c));
}

/** Parse the `cubic(x1,y1,x2,y2)` wire form. `x1`/`x2` clamp to `[0,1]` (keeps `x(t)` monotonic);
 *  `y1`/`y2` stay free so a curve can overshoot. Malformed input is `null`. */
export function parseCubic(s: string | undefined | null): [number, number, number, number] | null {
  const m = typeof s === "string" ? s.trim().match(/^cubic\(([^()]*)\)$/) : null;
  if (!m) return null;
  const parts = m[1].split(",");
  if (parts.length !== 4) return null;
  const v = parts.map((p) => Number(p.trim()));
  if (v.some((n) => !Number.isFinite(n)) || parts.some((p) => p.trim() === "")) return null;
  return [Math.min(1, Math.max(0, v[0])), v[1], Math.min(1, Math.max(0, v[2])), v[3]];
}

/** The canonical wire string - fixed 3-decimal fields, byte-identical to Rust's `format_cubic`,
 *  so a value written here survives `valid_easing`'s normalisation unchanged. */
export const formatCubic = (x1: number, y1: number, x2: number, y2: number) =>
  `cubic(${x1.toFixed(3)},${y1.toFixed(3)},${x2.toFixed(3)},${y2.toFixed(3)})`;
