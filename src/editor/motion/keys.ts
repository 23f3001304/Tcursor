import { parseCubic } from "../../shared/math/cubicBezier";

export type KeyMode = "b" | "h" | "l";
export interface Key {
  t: number;
  v: number;
  in: [number, number];
  out: [number, number];
  mode: KeyMode;
}
export interface Keys {
  keys: Key[];
}

export const KEYS_PREFIX = "keys(";
export const MAX_KEYS = 8;

const fr = Math.fround;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

const r3 = (x: number) => {
  const r = Math.round(x * 1000) / 1000;
  return r === 0 ? 0 : r;
};
const f3 = (x: number) => r3(x).toFixed(3);

const isMode = (s: string): s is KeyMode => s === "b" || s === "h" || s === "l";

export function parseKeys(s: string): Keys | null {
  const m = typeof s === "string" ? s.trim().match(/^keys\(([^()]*)\)$/) : null;
  if (!m) return null;
  const parts = m[1].split(",");
  if (parts.length < 2 || parts.length > MAX_KEYS) return null;
  const keys: Key[] = [];
  for (const part of parts) {
    const f = part.trim().split(/\s+/);
    if (f.length !== 7 || !isMode(f[6])) return null;
    const n = f.slice(0, 6).map(Number);
    if (n.some((x) => !Number.isFinite(x))) return null;
    keys.push({ t: n[0], v: n[1], in: [n[2], n[3]], out: [n[4], n[5]], mode: f[6] });
  }
  if (r3(keys[0].t) !== 0 || r3(keys[keys.length - 1].t) !== 1) return null;
  for (let i = 1; i < keys.length; i++) if (r3(keys[i].t) <= r3(keys[i - 1].t)) return null;
  return canonicalKeys({ keys });
}

export function canonicalKeys(k: Keys): Keys {
  const src = [...k.keys].sort((a, b) => a.t - b.t);
  const ts = src.map((x) => r3(x.t));
  const last = src.length - 1;
  return {
    keys: src.map((key, i): Key => ({
      t: ts[i],
      v: r3(key.v),
      in: i === 0 ? [0, 0] : [r3(clamp(key.in[0], -Math.max(0, ts[i] - ts[i - 1]), 0)), r3(key.in[1])],
      out: i === last ? [0, 0] : [r3(clamp(key.out[0], 0, Math.max(0, ts[i + 1] - ts[i]))), r3(key.out[1])],
      mode: key.mode,
    })),
  };
}

export function keysToString(k: Keys): string {
  const body = canonicalKeys(k)
    .keys.map(
      (x) => `${f3(x.t)} ${f3(x.v)} ${f3(x.in[0])} ${f3(x.in[1])} ${f3(x.out[0])} ${f3(x.out[1])} ${x.mode}`,
    )
    .join(",");
  return `${KEYS_PREFIX}${body})`;
}

function bez(p0: number, p1: number, p2: number, p3: number, s: number): number {
  const m = fr(1 - s);
  const a = fr(fr(m * m) * m),
    b = fr(fr(3 * fr(m * m)) * s);
  const c = fr(fr(3 * m) * fr(s * s)),
    d = fr(fr(s * s) * s);
  return fr(fr(fr(p0 * a) + fr(p1 * b)) + fr(fr(p2 * c) + fr(p3 * d)));
}

export function evalKeys(k: Keys, p: number): number {
  const ks = k.keys;
  if (ks.length === 0) return 0;
  if (ks.length === 1) return ks[0].v;
  const c = fr(clamp(p, 0, 1));
  let i = 0;
  while (i < ks.length - 2 && c >= ks[i + 1].t) i++;
  const a = ks[i],
    b = ks[i + 1];
  if (a.mode === "h") return a.v;
  const span = fr(b.t - a.t);
  const u = span > 0 ? fr(fr(c - a.t) / span) : 1;
  if (a.mode === "l") return fr(a.v + fr(fr(b.v - a.v) * u));
  const x1 = fr(a.t + a.out[0]),
    y1 = fr(a.v + a.out[1]);
  const x2 = fr(b.t + b.in[0]),
    y2 = fr(b.v + b.in[1]);
  let lo = 0,
    hi = 1;
  for (let n = 0; n < 20; n++) {
    const s = fr((lo + hi) * 0.5);
    if (bez(a.t, x1, x2, b.t, s) < c) lo = s;
    else hi = s;
  }
  return bez(a.v, y1, y2, b.v, fr((lo + hi) * 0.5));
}

const twoKey = (x1: number, y1: number, x2: number, y2: number, mode: KeyMode = "b"): Keys =>
  canonicalKeys({
    keys: [
      { t: 0, v: 0, in: [0, 0], out: [x1, y1], mode },
      { t: 1, v: 1, in: [x2 - 1, y2 - 1], out: [0, 0], mode },
    ],
  });

const EASE_IN_OUT_H = 0.45;

export function toKeys(easing: string): Keys | null {
  const k = parseKeys(easing);
  if (k) return k;
  const name = typeof easing === "string" ? easing.trim() : "";
  if (name === "linear") return twoKey(0, 0, 1, 1, "l");
  if (name === "smooth") return twoKey(1 / 3, 0, 2 / 3, 1);
  if (name === "ease_in") return twoKey(1 / 3, 0, 2 / 3, 1 / 3);
  if (name === "ease_out") return twoKey(1 / 3, 2 / 3, 2 / 3, 1);
  if (name === "ease_in_out") return twoKey(EASE_IN_OUT_H, 0, 1 - EASE_IN_OUT_H, 1);
  const c = parseCubic(name);
  return c ? twoKey(c[0], c[1], c[2], c[3]) : null;
}

export const isKeys = (easing: string | null | undefined): boolean =>
  typeof easing === "string" && easing.trim().startsWith(KEYS_PREFIX) && parseKeys(easing) !== null;
