import type { Zoom } from "../../lib/edit";

export interface FeelPreset { name: string; zoom_in_ms: number; zoom_out_ms: number; easing: string }

export const FEEL_PRESETS: FeelPreset[] = [
  { name: "Subtle", zoom_in_ms: 450, zoom_out_ms: 550, easing: "ease_in_out" },
  { name: "Balanced", zoom_in_ms: 350, zoom_out_ms: 450, easing: "smooth" },
  { name: "Punchy", zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" },
];

export function activeFeel(z: Pick<Zoom, "zoom_in_ms" | "zoom_out_ms" | "easing">): string | null {
  const hit = FEEL_PRESETS.find((p) => p.zoom_in_ms === z.zoom_in_ms
    && p.zoom_out_ms === z.zoom_out_ms && p.easing === z.easing);
  return hit ? hit.name : null;
}

export function feelPatch(name: string): Pick<Zoom, "zoom_in_ms" | "zoom_out_ms" | "easing"> | null {
  const p = FEEL_PRESETS.find((f) => f.name === name);
  return p ? { zoom_in_ms: p.zoom_in_ms, zoom_out_ms: p.zoom_out_ms, easing: p.easing } : null;
}
