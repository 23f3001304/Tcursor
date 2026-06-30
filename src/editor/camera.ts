import type { CamSample } from "../lib/ipc";

export interface Cam { scale: number; cx: number; cy: number; curx: number; cury: number }

/** Interpolate the camera (scale + center) and cursor position at time `ms` from the
 *  sampled track. Binary search for the bracketing samples, then lerp - so the composite
 *  is smooth between the per-frame samples even on a high-refresh display. */
export function camAt(track: CamSample[], ms: number): Cam {
  if (track.length === 0) return { scale: 1, cx: 0.5, cy: 0.5, curx: 0.5, cury: 0.5 };
  if (ms <= track[0].t) return track[0];
  const last = track[track.length - 1];
  if (ms >= last.t) return last;
  let lo = 0, hi = track.length - 1;
  while (lo + 1 < hi) { const mid = (lo + hi) >> 1; if (track[mid].t <= ms) lo = mid; else hi = mid; }
  const a = track[lo], b = track[hi];
  const f = b.t > a.t ? (ms - a.t) / (b.t - a.t) : 0;
  const mix = (x: number, y: number) => x + (y - x) * f;
  return {
    scale: mix(a.scale, b.scale), cx: mix(a.cx, b.cx), cy: mix(a.cy, b.cy),
    curx: mix(a.curx, b.curx), cury: mix(a.cury, b.cury),
  };
}
