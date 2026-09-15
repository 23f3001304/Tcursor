export const CAM_SNAP_MS = 80;

export function snapKeyframeMs(t: number, segEdges: number[], otherKfs: number[]): number {
  let best = t;
  let bestDist = Infinity;
  for (const c of [...segEdges, ...otherKfs]) {
    const d = Math.abs(c - t);
    if (d <= CAM_SNAP_MS && d < bestDist) {
      best = c;
      bestDist = d;
    }
  }
  return best;
}
