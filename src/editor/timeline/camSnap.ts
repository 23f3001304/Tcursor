/** How close (ms) a dragged camera keyframe must get to a snap target before it locks on.
 *  Measured in MILLISECONDS, not pixels, so the feel is identical at every timeline zoom. */
export const CAM_SNAP_MS = 80;

/** Snaps a dragged camera keyframe's time to the nearest layout-segment edge or other keyframe
 *  within `CAM_SNAP_MS`, else returns `t` unchanged. Pure (no DOM, no doc mutation) so the lane
 *  can call it on every pointermove. Ties go to the first candidate in `segEdges` then
 *  `otherKfs`, which is stable because both lists come from the doc in a fixed order.
 *
 *  Why these two target sets: a keyframe's whole purpose after Task 27 is to own a span that
 *  butts against the layout segments around it, so segment starts/ends and sibling keyframes are
 *  the only times worth landing on exactly. The dragged keyframe's OWN time is never a candidate
 *  (the caller filters it out), or it would snap to where it started and refuse to move. */
export function snapKeyframeMs(t: number, segEdges: number[], otherKfs: number[]): number {
  let best = t;
  let bestDist = Infinity;
  for (const c of [...segEdges, ...otherKfs]) {
    const d = Math.abs(c - t);
    if (d <= CAM_SNAP_MS && d < bestDist) { best = c; bestDist = d; }
  }
  return best;
}
