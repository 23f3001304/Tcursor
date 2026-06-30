import type { EffectRegion } from "../lib/edit";
import type { HoldSpan } from "../lib/ipc";

const FADE = 250; // ms, matches the export's FADE_MS

/** Fade-in/out ramp 0..1 over a `[start, end)` interval (250ms each side, matching the export). */
function ramp(start: number, end: number, ms: number): number {
  if (ms < start || ms >= end) return 0;
  return Math.min(1, Math.max(0, Math.min((ms - start) / FADE, (end - ms) / FADE)));
}

/** Spotlight strength 0..1 at output time `ms`: the max fade ramp over any editable Spotlight
 *  region AND any recorded hotkey-hold span, unioned with the global settings toggle. Mirrors the
 *  export's `s_alpha = max(settings, hold_alpha(actions), region_alpha)` so the preview matches. */
export function spotlightAlpha(effects: EffectRegion[], holds: HoldSpan[], settingsOn: boolean, ms: number): number {
  let a = settingsOn ? 1 : 0;
  for (const e of effects) if (e.kind === "spotlight") a = Math.max(a, ramp(e.start_ms, e.end_ms, ms));
  for (const h of holds) a = Math.max(a, ramp(h.start_ms, h.end_ms, ms));
  return a;
}

export interface SpotParams { dim: number; radius: number; feather: number }

/** Live spotlight input for the preview: the editable regions + the recorded hotkey holds + the
 *  global settings toggle + the dim params (from the recording's clickfx). `null` when disabled. */
export interface SpotlightInput { effects: EffectRegion[]; holds: HoldSpan[]; on: boolean; params: SpotParams }

/** Draw the spotlight over the whole canvas with a soft hole at the cursor (`p`), approximating
 *  the export's Classic spotlight: a colorless radial darken (toward black), transparent within
 *  `radius * h` and reaching `dim * alpha` past `(radius + feather) * h`. No-op at `alpha <= 0`.
 *  Other modes (Vignette/Nebula/Halo/Blur/Breathing) and hotkey holds are not yet previewed. */
export function drawSpotlight(ctx: CanvasRenderingContext2D, w: number, h: number,
  p: [number, number], alpha: number, sp: SpotParams) {
  if (alpha <= 0) return;
  const r0 = Math.max(0, sp.radius * h), r1 = r0 + Math.max(1, sp.feather * h);
  const g = ctx.createRadialGradient(p[0], p[1], r0, p[0], p[1], r1);
  g.addColorStop(0, "rgba(0,0,0,0)");
  g.addColorStop(1, `rgba(0,0,0,${sp.dim * alpha})`);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
}
