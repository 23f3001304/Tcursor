import { keysToString, parseKeys } from "./keys";

export interface MotionPreset {
  id: "snappy" | "soft" | "cinematic" | "mechanical" | "bouncy";
  name: string;
  easing: string;
  easing_out: string;
  feel: string;
}

export const PRESETS: MotionPreset[] = [
  {
    id: "snappy",
    name: "Snappy",
    easing: "keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)",
    easing_out: "keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)",
    feel: "Leaves fast, settles long",
  },
  {
    id: "soft",
    name: "Soft",
    easing: "smooth",
    easing_out: "smooth",
    feel: "The default. Eases in, eases out",
  },
  {
    id: "cinematic",
    name: "Cinematic",
    easing: "keys(0 0 0 0 0.45 0 b,1 1 -0.25 0 0 0 b)",
    easing_out: "keys(0 0 0 0 0.15 -0.06 b,1 1 -0.4 0 0 0 b)",
    feel: "Slow start, long deceleration, a breath before the zoom out",
  },
  {
    id: "mechanical",
    name: "Mechanical",
    easing: "keys(0 0 0 0 0 0 l,0.85 1 0 0 0 0 h,1 1 0 0 0 0 l)",
    easing_out: "keys(0 0 0 0 0 0 l,0.85 1 0 0 0 0 h,1 1 0 0 0 0 l)",
    feel: "Constant speed, then a short hold",
  },
  {
    id: "bouncy",
    name: "Bouncy",
    easing: "spring(140,7,1)",
    easing_out: "spring(140,7,1)",
    feel: "A real damped oscillator, overshoots about 1.2",
  },
];

const canonical = (s: string): string => {
  const k = parseKeys(s);
  if (k) return keysToString(k);
  const m = /^spring\(\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/.exec(s.trim());
  if (!m) return s.trim();
  const n = (v: string | undefined, dflt: number) => (v === undefined ? dflt : Number(v)).toFixed(3);
  return `spring(${n(m[1], 0)},${n(m[2], 0)},${n(m[3], 1)})`;
};

export function presetOf(
  easing: string,
  easingOut: string | null | undefined,
): MotionPreset["id"] | "custom" {
  const i = canonical(easing);
  const o = easingOut ? canonical(easingOut) : i;
  return PRESETS.find((p) => canonical(p.easing) === i && canonical(p.easing_out) === o)?.id ?? "custom";
}

export function presetPatch(id: string): { easing: string; easing_out: string } {
  const p = PRESETS.find((x) => x.id === id) ?? PRESETS.find((x) => x.id === "soft")!;
  return { easing: p.easing, easing_out: p.easing_out };
}
