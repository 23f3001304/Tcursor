// The processing/export wave: one sine sweeping left to right with the dot as its scanning head.
// Indeterminate work (the AI director) drives the head from a clock; an export drives it from
// percent complete, and the whole wave's amplitude decays to flat as that percent reaches 100.

/** Seconds for one indeterminate left-to-right pass. */
export const SWEEP_PERIOD_S = 1.8;
/** How much of the width the wave trails behind the head, as a fraction of the width. */
export const TRAIL = 0.34;

/** The head's position (0..1) at `t` seconds - a sawtooth, so the pass restarts at the left edge
 *  rather than bouncing back (a bounce reads as "undoing work"). */
export function sweepHead(t: number, period = SWEEP_PERIOD_S): number {
  if (!(period > 0)) return 0;
  const x = (t / period) % 1;
  return x < 0 ? x + 1 : x;
}

/** The amplitude envelope at normalised x for a head at `head`: full height right behind the
 *  head, fading to nothing one `trail` further back, and flat ahead of the head - the wave only
 *  exists where work has already passed. */
export function sweepEnvelope(x: number, head: number, trail = TRAIL): number {
  if (x > head) return 0;
  if (!(trail > 0)) return 0;
  const behind = (head - x) / trail;
  if (behind >= 1) return 0;
  // Raised cosine, so the trailing edge dies out smoothly instead of ending on a visible corner.
  return 0.5 * (1 + Math.cos(Math.PI * behind));
}

/** The global amplitude multiplier for an export at `pct` percent: 1 at the start, flat at 100%.
 *  Clamped, so a stale or out-of-range percent can never invert or overshoot the decay. */
export function progressAmp(pct: number): number {
  return 1 - Math.max(0, Math.min(100, pct)) / 100;
}

/** Where the head sits for a given frame: from `pct` when the caller knows it (the dot doubles as
 *  the percentage marker), otherwise swept from the clock. */
export function headFor(t: number, pct: number | undefined): number {
  return pct === undefined ? sweepHead(t) : Math.max(0, Math.min(100, pct)) / 100;
}
