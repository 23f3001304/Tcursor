const TILE_ASPECT = 16 / 9;

export const FILMSTRIP_HEIGHT = 80;

export const EDITOR_TRACK_W = 1440 - 88 - 16;

export function filmstripCount(trackWidthPx: number, tileHeightPx: number): number {
  const slots = Math.round(trackWidthPx / (tileHeightPx * TILE_ASPECT));
  return Math.min(24, Math.max(8, slots));
}

export const FILMSTRIP_COUNT = filmstripCount(EDITOR_TRACK_W, FILMSTRIP_HEIGHT);
