/** Minimum on-screen movement (px) before a pointer drag counts as a real drag rather than a
 *  click - shared by every timeline/stage drag that must not commit a no-op edit (an undo step +
 *  an IPC round trip) just because the user selected something (D-Medium M8; UX audit #5 - a
 *  stationary click on a spotlight pill silently nudged its Start 0 -> 0.03s). Close to
 *  `CamDragHandle`'s existing 4px guard without being tied to its exact number. */
export const DRAG_THRESHOLD_PX = 3;

/** Pure hit-test: has the pointer moved far enough from its drag-start position to count as an
 *  actual drag? Euclidean distance (not axis-only), so a diagonal jiggle under the threshold is
 *  still ignored even when its X or Y component alone would exceed it. */
export function pastDragThreshold(dx: number, dy: number, px: number = DRAG_THRESHOLD_PX): boolean {
  return Math.hypot(dx, dy) >= px;
}
