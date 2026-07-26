/** Rough ETA (ms remaining) from elapsed time and percent complete, assuming a constant encode
 *  rate: `elapsed / pct * (100 - pct)`. Returns `null` before there is enough signal (`pct <= 0`)
 *  to avoid a division-by-zero / a wild early estimate that would flicker as the first frames land. */
export function estimateEtaMs(elapsedMs: number, pct: number): number | null {
  if (pct <= 0) return null;
  const totalMs = (elapsedMs / pct) * 100;
  return Math.max(0, totalMs - elapsedMs);
}
