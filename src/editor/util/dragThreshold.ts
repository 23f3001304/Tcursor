export const DRAG_THRESHOLD_PX = 3;

export function pastDragThreshold(dx: number, dy: number, px: number = DRAG_THRESHOLD_PX): boolean {
  return Math.hypot(dx, dy) >= px;
}
