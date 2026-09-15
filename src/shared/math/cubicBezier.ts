const bez = (a: number, b: number, t: number) => {
  const c1 = 3 * a,
    c2 = 3 * b - 6 * a,
    c3 = 3 * a - 3 * b + 1;
  return ((c3 * t + c2) * t + c1) * t;
};
const dbez = (a: number, b: number, t: number) => {
  const c1 = 3 * a,
    c2 = 3 * b - 6 * a,
    c3 = 3 * a - 3 * b + 1;
  return (3 * c3 * t + 2 * c2) * t + c1;
};

function solveT(x1: number, x2: number, x: number): number {
  let t = x;
  for (let i = 0; i < 8; i++) {
    const e = bez(x1, x2, t) - x;
    if (Math.abs(e) < 1e-7) return t;
    const d = dbez(x1, x2, t);
    if (Math.abs(d) < 1e-6) break;
    t = Math.min(1, Math.max(0, t - e / d));
  }
  let lo = 0,
    hi = 1;
  t = x;
  for (let i = 0; i < 30; i++) {
    const e = bez(x1, x2, t) - x;
    if (Math.abs(e) < 1e-7) return t;
    if (e > 0) hi = t;
    else lo = t;
    t = (lo + hi) / 2;
  }
  return t;
}

export function evalCubic(x1: number, y1: number, x2: number, y2: number, p: number): number {
  const c = Math.min(1, Math.max(0, p));
  if (c <= 0) return 0;
  if (c >= 1) return 1;
  return bez(y1, y2, solveT(x1, x2, c));
}

export function parseCubic(s: string | undefined | null): [number, number, number, number] | null {
  const m = typeof s === "string" ? s.trim().match(/^cubic\(([^()]*)\)$/) : null;
  if (!m) return null;
  const parts = m[1].split(",");
  if (parts.length !== 4) return null;
  const v = parts.map((p) => Number(p.trim()));
  if (v.some((n) => !Number.isFinite(n)) || parts.some((p) => p.trim() === "")) return null;
  return [Math.min(1, Math.max(0, v[0])), v[1], Math.min(1, Math.max(0, v[2])), v[3]];
}

export const formatCubic = (x1: number, y1: number, x2: number, y2: number) =>
  `cubic(${x1.toFixed(3)},${y1.toFixed(3)},${x2.toFixed(3)},${y2.toFixed(3)})`;
