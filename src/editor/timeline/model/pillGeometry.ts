export const PILL_EDGE_INSET_PCT = 0.6;

export const MIN_PILL_PCT = 2.5;

export const PILL_GAP_PCT = 0.15;

export function pillLeftPct(startMs: number, durMs: number): number {
  if (durMs <= 0) return 0;
  if (startMs <= 0) return PILL_EDGE_INSET_PCT;
  return (startMs / durMs) * 100;
}

export function pillWidthPct(
  startMs: number,
  endMs: number,
  durMs: number,
  leftPct = pillLeftPct(startMs, durMs),
  nextStartMs?: number,
): number {
  if (durMs <= 0) return 0;
  const rawWidthPct = ((endMs - startMs) / durMs) * 100;
  const maxWidthPct = Math.max(0, 100 - PILL_EDGE_INSET_PCT - leftPct);
  // The floor keeps a tiny pill draggable, but never past the next pill in its row: a dense
  // lane (87 captions) must read as adjacent blocks, not as blocks lying on top of each other.
  const roomPct =
    nextStartMs === undefined ? Infinity : ((nextStartMs - startMs) / durMs) * 100 - PILL_GAP_PCT;
  return Math.max(Math.min(rawWidthPct, maxWidthPct), Math.min(MIN_PILL_PCT, roomPct));
}
