import { factorAt, gapContaining, type TimeMap } from "../../../shared/math/remap";

export const FRAME_MS = 1000 / 60;

export interface PlaybackAction {
  seekTo: number | null;
  rate: number;
}

export function playbackAction(map: TimeMap, tMs: number): PlaybackAction {
  const gap = gapContaining(map, tMs);
  return { seekTo: gap && Number.isFinite(gap[1]) ? gap[1] : null, rate: factorAt(map, tMs) };
}

/** True when a jump from `prevMs` to `tMs` is the preview skipping a cut (the tick that triggered the
 *  seek saw a clip time at or just past the cut's start, and this tick landed on its end), so the
 *  trail and spotlight sims must NOT reset the way they do for a real seek: the export never saw
 *  those frames either and keeps its state across them. */
export function isCutJump(map: TimeMap, prevMs: number, tMs: number, frameMs = FRAME_MS): boolean {
  const segs = map.segments;
  for (let i = 1; i < segs.length; i++) {
    const a = segs[i - 1].clipEnd,
      b = segs[i].clipStart;
    if (prevMs >= a - 2 * frameMs && prevMs <= a + 2 * frameMs && Math.abs(tMs - b) <= 2 * frameMs)
      return true;
  }
  return false;
}

export const NATURAL_TICK_MAX_DELTA_MS = 500;

export function isNaturalPlaybackTick(
  prevMs: number,
  nextMs: number,
  playing: boolean,
  maxDeltaMs: number = NATURAL_TICK_MAX_DELTA_MS,
): boolean {
  if (!playing) return false;
  const delta = nextMs - prevMs;
  return delta >= 0 && delta <= maxDeltaMs;
}
