export function estimateEtaMs(elapsedMs: number, pct: number): number | null {
  if (pct <= 0) return null;
  const totalMs = (elapsedMs / pct) * 100;
  return Math.max(0, totalMs - elapsedMs);
}
