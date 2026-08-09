// Pure state -> animation-config mapping for the living TcursorMark (Task 39). Kept separate from
// TcursorMark.tsx itself so the mapping is testable without touching React/Motion at all.

export type MarkState = "idle" | "recording" | "exporting" | "directing";

/** The wave path's horizontal repeat period, in the SAME SVG user units as `WAVE_D` - a clipped
 *  translate by exactly this much loops seamlessly (verified against the path's own geometry at
 *  dispatch). `TcursorMark` tiles enough copies of the path, spaced one `WAVE_LAMBDA` apart, to
 *  cover the viewBox across the whole animated range, not just its two endpoints. */
export const WAVE_LAMBDA = 88;

/** Seconds for one full λ-wide flow loop in `state` - `0` means "don't animate" (idle, or the
 *  caller's own reduced-motion/settings gate). `exporting` speeds up as `pct` climbs toward 100 (a
 *  satisfying "picking up speed" read as the export nears done): linearly interpolated from 3s/λ
 *  at 0% down to 0.8s/λ at 100%, clamped to that range for any `pct` outside 0..100 (design pass -
 *  the brief asked for pace mapped from export progress; a direct pct->pace curve was chosen over
 *  measuring a literal frame-to-frame percent velocity, which would need extra cross-render state
 *  for a purely cosmetic knob). `recording`/`directing` flow at a fixed ~2s/λ.
 */
export function flowSeconds(state: MarkState, pct = 0): number {
  if (state === "idle") return 0;
  if (state === "exporting") {
    const t = Math.max(0, Math.min(100, pct)) / 100;
    return 3 - t * (3 - 0.8); // 3s/λ at 0% -> 0.8s/λ at 100%
  }
  return 2; // recording / directing
}

/** Whether the REC dot should pulse (Motion spring scale/opacity loop) in `state` - recording only. */
export function dotPulses(state: MarkState): boolean {
  return state === "recording";
}

/** The dot's color override for `state`, or `null` to keep the caller's own `dotColor` prop. Only
 *  `directing` retints the dot (to `--e-ai`) - the wave itself always stays `currentColor` in
 *  every state, so the mark never stops reading as "the same mark", just differently animated. */
export function dotTint(state: MarkState): string | null {
  return state === "directing" ? "var(--e-ai)" : null;
}
