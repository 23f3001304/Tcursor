export const SWEEP_PERIOD_S = 1.8;

export const TRAIL = 0.34;

export function sweepHead(t: number, period = SWEEP_PERIOD_S): number {
  if (!(period > 0)) return 0;
  const x = (t / period) % 1;
  return x < 0 ? x + 1 : x;
}

export function sweepEnvelope(x: number, head: number, trail = TRAIL): number {
  if (x > head) return 0;
  if (!(trail > 0)) return 0;
  const behind = (head - x) / trail;
  if (behind >= 1) return 0;
  return 0.5 * (1 + Math.cos(Math.PI * behind));
}

export function progressAmp(pct: number): number {
  return 1 - Math.max(0, Math.min(100, pct)) / 100;
}

export function headFor(t: number, pct: number | undefined): number {
  return pct === undefined ? sweepHead(t) : Math.max(0, Math.min(100, pct)) / 100;
}
