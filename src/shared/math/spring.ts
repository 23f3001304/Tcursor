export const SPRING_RANGE = { stiffness: [1, 2000], damping: [0, 200], mass: [0.1, 10] } as const;

export const SPRING_DEFAULT: [number, number, number] = [100, 10, 1];

const ZETA_MIN = 0.05;
const LN_EPS = 6.907755;
const CRIT = 1e-3;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

function zeta(stiffness: number, damping: number, mass: number): number {
  const k = clamp(stiffness, 1, 2000),
    c = clamp(damping, 0, 200),
    m = clamp(mass, 0.1, 10);
  return Math.max(ZETA_MIN, c / (2 * Math.sqrt(k * m)));
}

const settleU = (z: number) => LN_EPS / (z <= 1 ? z : z - Math.sqrt(Math.max(0, z * z - 1)));

function resp(z: number, u: number): number {
  if (Math.abs(z - 1) <= CRIT) return 1 - (1 + u) * Math.exp(-u);
  if (z < 1) {
    const d = Math.sqrt(1 - z * z);
    return 1 - Math.exp(-z * u) * (Math.cos(d * u) + (z / d) * Math.sin(d * u));
  }
  const s = Math.sqrt(Math.max(0, z * z - 1));
  const r1 = -(z - s),
    r2 = -(z + s);
  return 1 - (r2 * Math.exp(r1 * u) - r1 * Math.exp(r2 * u)) / (r2 - r1);
}

export function spring(stiffness: number, damping: number, mass: number, p: number): number {
  if (!Number.isFinite(p) || p <= 0) return 0;
  if (p >= 1) return 1;
  const z = zeta(stiffness, damping, mass),
    us = settleU(z);
  return resp(z, p * us) + p * (1 - resp(z, us));
}

export function parseSpring(s: string | undefined | null): [number, number, number] | null {
  const m = typeof s === "string" ? s.trim().match(/^spring\(([^()]*)\)$/) : null;
  if (!m) return null;
  const parts = m[1].split(",");
  if (parts.length < 2 || parts.length > 3) return null;
  const v = parts.map((x) => Number(x.trim()));
  if (v.some((n) => !Number.isFinite(n)) || parts.some((x) => x.trim() === "")) return null;
  return [clamp(v[0], 1, 2000), clamp(v[1], 0, 200), clamp(v[2] ?? 1, 0.1, 10)];
}

export const formatSpring = (stiffness: number, damping: number, mass = 1) =>
  `spring(${stiffness.toFixed(3)},${damping.toFixed(3)},${mass.toFixed(3)})`;

export function springOf(easing: string | undefined | null): [number, number, number] | null {
  if (easing === "spring") return [...SPRING_DEFAULT];
  return parseSpring(easing);
}
