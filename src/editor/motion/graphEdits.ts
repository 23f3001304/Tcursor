import {
  canonicalKeys,
  evalKeys,
  keysToString,
  MAX_KEYS,
  parseKeys,
  toKeys,
  type Key,
  type Keys,
} from "./keys";
import type { GraphInput, RampSpec } from "./graphModel";

export interface GraphPatch {
  easing?: string;
  easing_out?: string;
  inMs?: number;
  outMs?: number;
}

const T_EPS = 0.001;

export const MIN_RAMP_MS = 0;
export const NUDGE = 0.01;
export const NUDGE_BIG = 0.1;

export function toKeysInput(input: GraphInput): GraphInput {
  const conv = (s: RampSpec | null) => {
    if (!s || parseKeys(s.easing)) return s;
    const k = toKeys(s.easing);
    return k ? { ...s, easing: keysToString(k) } : s;
  };
  return { ...input, rampIn: conv(input.rampIn)!, rampOut: conv(input.rampOut) };
}

export function moveKey(k: Keys, i: number, t: number, v: number): Keys {
  const ks = k.keys;
  const at =
    i === 0 ? 0 : i === ks.length - 1 ? 1 : Math.min(ks[i + 1].t - T_EPS, Math.max(ks[i - 1].t + T_EPS, t));
  return canonicalKeys({ keys: ks.map((key, n) => (n === i ? { ...key, t: at, v } : key)) });
}

export function moveHandle(k: Keys, i: number, side: "in" | "out", dx: number, dy: number): Keys {
  const h: [number, number] = [dx, dy];
  return canonicalKeys({
    keys: k.keys.map((key, n) =>
      n === i ? { ...key, in: side === "in" ? h : key.in, out: side === "out" ? h : key.out } : key,
    ),
  });
}

export function addKeyAt(k: Keys, p: number): Keys {
  const ks = k.keys;
  if (ks.length >= MAX_KEYS) return k;
  const t = Math.min(1 - T_EPS, Math.max(T_EPS, p));
  let i = 0;
  while (i < ks.length - 2 && ks[i + 1].t <= t) i++;
  if (Math.abs(ks[i].t - t) < T_EPS || Math.abs(ks[i + 1].t - t) < T_EPS) return k;
  const add: Key = { t, v: evalKeys(k, t), in: [0, 0], out: [0, 0], mode: ks[i].mode };
  return canonicalKeys({ keys: [...ks.slice(0, i + 1), add, ...ks.slice(i + 1)] });
}

export function removeKey(k: Keys, i: number): Keys {
  if (i <= 0 || i >= k.keys.length - 1) return k;
  return canonicalKeys({ keys: k.keys.filter((_, n) => n !== i) });
}

export function nudgeKey(k: Keys, i: number, dt: number, dv: number, big = false): Keys {
  const s = big ? NUDGE_BIG : NUDGE;
  return moveKey(k, i, k.keys[i].t + dt * s, k.keys[i].v + dv * s);
}

export const retimeIndex = (which: "in" | "out", n: number) => (which === "in" ? n - 1 : 0);

export function retimeMs(input: GraphInput, which: "in" | "out", ms: number): number {
  const span = Math.max(1, input.endMs - input.startMs);
  const raw = which === "in" ? ms - input.startMs : input.endMs - ms;
  return Math.round(Math.min(span, Math.max(MIN_RAMP_MS, raw)));
}

export function withCurve(input: GraphInput, which: "in" | "out", k: Keys, durMs?: number): GraphInput {
  const cur = which === "in" ? input.rampIn : input.rampOut;
  const spec: RampSpec = { easing: keysToString(k), durMs: durMs ?? cur?.durMs ?? 0 };
  return which === "in" ? { ...input, rampIn: spec } : { ...input, rampOut: spec };
}

export const patchOf = (which: "in" | "out", k: Keys, durMs?: number): GraphPatch =>
  which === "in"
    ? { easing: keysToString(k), ...(durMs === undefined ? {} : { inMs: durMs }) }
    : { easing_out: keysToString(k), ...(durMs === undefined ? {} : { outMs: durMs }) };
