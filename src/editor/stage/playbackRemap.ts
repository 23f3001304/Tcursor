import { factorAt, gapContaining, type TimeMap } from "../../lib/remap";

/** One output frame at 60fps; the tolerance every rule below is measured in. */
export const FRAME_MS = 1000 / 60;

/** What the media elements must do at clip time `tMs` while playing. */
export interface PlaybackAction {
  /** Jump every media element here (a cut's end, or the trim-in): the viewer never sees the gap. */
  seekTo: number | null;
  /** `playbackRate` for the speed span containing `tMs`, 1 outside every span. */
  rate: number;
}

/** Inside a finite gap (a cut, or before the trim-in) the media jump to its end; the trailing gap
 *  past the trim-out has no end to jump to, so playback simply runs out as it always has. The rate
 *  is the containing segment's factor (Chromium keeps the pitch). */
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
    const a = segs[i - 1].clipEnd, b = segs[i].clipStart;
    if (prevMs >= a - 2 * frameMs && prevMs <= a + 2 * frameMs && Math.abs(tMs - b) <= 2 * frameMs) return true;
  }
  return false;
}
