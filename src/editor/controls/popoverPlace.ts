/** Viewport geometry for a floating layer that is PORTALLED out of its anchor instead of being
 *  positioned inside it. Clipping is DOM containment, not geometry: an `overflow: hidden` or
 *  `overflow: auto` ancestor cuts off an absolutely positioned descendant no matter where it is
 *  drawn, and the editor is full of them (`.e-panel`'s scroll box, `.e-panel-slot`, `.e-props-side`,
 *  `.e-tracks`, the tile strips). Moving the layer out and placing it here is the only fix that
 *  holds for every caller. Pure, so the flip rules are pinned by `popoverPlace.test.ts` rather than
 *  by opening a menu at the bottom of a panel and looking. */

/** A viewport-space box, i.e. the part of `getBoundingClientRect()` placement actually reads. */
export interface Rect { left: number; top: number; width: number; height: number }

/** Where to paint a floating layer, in viewport coordinates (`position: fixed`). `flipped` is true
 *  when the preferred side had no room and the opposite one was used, so the caller can point its
 *  enter animation the other way. */
export interface Placement { left: number; top: number; flipped: boolean }

/** How close to the window edge a floating layer is allowed to sit. */
export const EDGE_MARGIN = 6;

/** Clamped into `[lo, hi]`, with `lo` winning when the layer is bigger than the space it has: a
 *  too-tall menu should hang off the bottom, never off the top where its first row is unreachable. */
const clamp = (v: number, lo: number, hi: number) => (hi < lo ? lo : Math.min(hi, Math.max(lo, v)));

/** Beside the anchor and vertically centred on it: to the RIGHT by default, flipped to the left
 *  when the right side would leave the window and the left side has room. What `Tooltip` wants -
 *  the rail sits against the window's left edge, so its labels always take the right side. */
export function placeBeside(anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 8): Placement {
  const right = anchor.left + anchor.width + gap;
  const flipped = right + w > vw - EDGE_MARGIN && anchor.left - gap - w >= EDGE_MARGIN;
  return {
    left: clamp(flipped ? anchor.left - gap - w : right, EDGE_MARGIN, vw - EDGE_MARGIN - w),
    top: clamp(anchor.top + anchor.height / 2 - h / 2, EDGE_MARGIN, vh - EDGE_MARGIN - h),
    flipped,
  };
}

/** Above or below the anchor, left edges aligned: `preferAbove` picks the starting side and the
 *  other one is used when the first does not fit. What `Picker`'s menu wants (below, flipping up
 *  near the foot of a panel or the inspector column) and what the camera lane's curve popover wants
 *  (above, flipping down when the Camera lane is the first lane in the stack). */
export function placeStacked(
  anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 4, preferAbove = false
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

/** Same as `placeStacked`, but CENTRED on the anchor rather than left-aligned with it - a popover
 *  that points at a mark (the camera lane's transition segment) belongs over its middle. */
export function placeStackedCentred(
  anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 4, preferAbove = false
): Placement {
  const centred = { ...anchor, left: anchor.left + anchor.width / 2 - w / 2, width: w };
  return placeStacked(centred, w, h, vw, vh, gap, preferAbove);
}

/** The element a portalled popover mounts into: the editor's own root, NOT `document.body`. The
 *  `--e-*` palette is declared on `.editor` (see `editor.css`'s header) and custom properties
 *  inherit down the DOM, so a layer parented to `document.body` would resolve every colour it
 *  paints to nothing. `.editor` is one node above every overflow container in the editor, so
 *  mounting there escapes all of them; and the layer itself is `position: fixed`, whose containing
 *  block is the viewport, so `.editor`'s own `overflow: hidden` does not clip it either. Falls back
 *  to `document.body` for a caller rendered outside the editor (component tests). */
export function portalHost(from: Element | null): Element {
  return from?.closest(".editor") ?? document.body;
}
