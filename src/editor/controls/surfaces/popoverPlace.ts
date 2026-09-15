export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface Placement {
  left: number;
  top: number;
  flipped: boolean;
}

export const EDGE_MARGIN = 6;

const clamp = (v: number, lo: number, hi: number) => (hi < lo ? lo : Math.min(hi, Math.max(lo, v)));

export function placeBeside(anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 8): Placement {
  const right = anchor.left + anchor.width + gap;
  const flipped = right + w > vw - EDGE_MARGIN && anchor.left - gap - w >= EDGE_MARGIN;
  return {
    left: clamp(flipped ? anchor.left - gap - w : right, EDGE_MARGIN, vw - EDGE_MARGIN - w),
    top: clamp(anchor.top + anchor.height / 2 - h / 2, EDGE_MARGIN, vh - EDGE_MARGIN - h),
    flipped,
  };
}

export function placeStacked(
  anchor: Rect,
  w: number,
  h: number,
  vw: number,
  vh: number,
  gap = 4,
  preferAbove = false,
): Placement {
  const below = anchor.top + anchor.height + gap;
  const above = anchor.top - gap - h;
  const fitsBelow = below + h <= vh - EDGE_MARGIN;
  const fitsAbove = above >= EDGE_MARGIN;
  const useAbove = preferAbove ? fitsAbove || !fitsBelow : !fitsBelow && fitsAbove;
  return {
    left: clamp(anchor.left, EDGE_MARGIN, vw - EDGE_MARGIN - w),
    top: clamp(useAbove ? above : below, EDGE_MARGIN, vh - EDGE_MARGIN - h),
    flipped: useAbove !== preferAbove,
  };
}

export function placeStackedCentred(
  anchor: Rect,
  w: number,
  h: number,
  vw: number,
  vh: number,
  gap = 4,
  preferAbove = false,
): Placement {
  const centred = { ...anchor, left: anchor.left + anchor.width / 2 - w / 2, width: w };
  return placeStacked(centred, w, h, vw, vh, gap, preferAbove);
}

export function portalHost(from: Element | null): Element {
  return from?.closest(".editor") ?? document.body;
}
