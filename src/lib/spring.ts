/** TS mirror of the Rust `export::spring` module - the SAME math, because the export is the source
 *  of truth and this only drives the live preview. Every change here needs the matching change
 *  there (and vice versa); `spring.test.ts` pins the shared parity table on this side.
 *  The semantics (why `p` is remapped onto the spring's own settle time, and what that costs) are
 *  documented once, in `docs/api/src-tauri/src/export/spring.md`. */

export const SPRING_PREFIX = "spring(";
/** Ranges the wire and the UI agree on - mirrors `spring.rs`'s consts. */
export const SPRING_RANGE = { stiffness: [1, 2000], damping: [0, 200], mass: [0.1, 10] } as const;
/** What the bare wire word `"spring"` means (Rust's `SPRING_DEFAULT`): Motion's own default. */
export const SPRING_DEFAULT: [number, number, number] = [100, 10, 1];

const ZETA_MIN = 0.05;
const LN_EPS = 6.907755; // ln(1 / 1e-3)
const CRIT = 1e-3;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** The damping ratio the curve's shape is entirely a function of (see `spring`). */
function zeta(stiffness: number, damping: number, mass: number): number {
  const k = clamp(stiffness, 1, 2000), c = clamp(damping, 0, 200), m = clamp(mass, 0.1, 10);
  return Math.max(ZETA_MIN, c / (2 * Math.sqrt(k * m)));
}

/** `w0 * t` at which the slowest mode has decayed to 1e-3 - the instant `p = 1` maps to. */
const settleU = (z: number) => LN_EPS / (z <= 1 ? z : z - Math.sqrt(Math.max(0, z * z - 1)));

/** Unit-step response of the oscillator at `u = w0 * t`, from rest at 0 toward 1. */
function resp(z: number, u: number): number {
  if (Math.abs(z - 1) <= CRIT) return 1 - (1 + u) * Math.exp(-u);
  if (z < 1) {
    const d = Math.sqrt(1 - z * z);
    return 1 - Math.exp(-z * u) * (Math.cos(d * u) + (z / d) * Math.sin(d * u));
  }
  const s = Math.sqrt(Math.max(0, z * z - 1));
  const r1 = -(z - s), r2 = -(z + s);
  return 1 - (r2 * Math.exp(r1 * u) - r1 * Math.exp(r2 * u)) / (r2 - r1);
}

/** The spring easing at progress `p`. Exact 0 at `p <= 0` and exact 1 at `p >= 1`; underdamped
 *  params overshoot past 1 on the way, critical and overdamped ones are monotone. */
export function spring(stiffness: number, damping: number, mass: number, p: number): number {
  if (!Number.isFinite(p) || p <= 0) return 0;
  if (p >= 1) return 1;
  const z = zeta(stiffness, damping, mass), us = settleU(z);
  return resp(z, p * us) + p * (1 - resp(z, us));
}

/** Parse `spring(stiffness,damping)` or `spring(stiffness,damping,mass)`; mass defaults to 1 and
 *  values clamp into the published ranges. Malformed or wrong-arity input is `null`. */
export function parseSpring(s: string | undefined | null): [number, number, number] | null {
  const m = typeof s === "string" ? s.trim().match(/^spring\(([^()]*)\)$/) : null;
  if (!m) return null;
  const parts = m[1].split(",");
  if (parts.length < 2 || parts.length > 3) return null;
  const v = parts.map((x) => Number(x.trim()));
  if (v.some((n) => !Number.isFinite(n)) || parts.some((x) => x.trim() === "")) return null;
  return [clamp(v[0], 1, 2000), clamp(v[1], 0, 200), clamp(v[2] ?? 1, 0.1, 10)];
}

/** The canonical wire string - fixed 3-decimal fields and always all three, byte-identical to
 *  Rust's `format_spring`, so a value written here survives `valid_easing` unchanged. */
export const formatSpring = (stiffness: number, damping: number, mass = 1) =>
  `spring(${stiffness.toFixed(3)},${damping.toFixed(3)},${mass.toFixed(3)})`;

/** The parameters an easing wire-name means, or `null` when it is not a spring at all: the bare
 *  word `"spring"` is `SPRING_DEFAULT`, `spring(...)` carries its own. The one place the UI and
 *  `ease` both ask "is this a spring, and which one". */
export function springOf(easing: string | undefined | null): [number, number, number] | null {
  if (easing === "spring") return [...SPRING_DEFAULT];
  return parseSpring(easing);
}
