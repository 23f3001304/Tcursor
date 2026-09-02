/** Default slack (ms) above the composite loop's own throttled report cadence
 *  (`useCompositeLoop.ts`'s `t - lastReportRef.current >= 60`) - generous enough to absorb a
 *  janky/backgrounded-tab frame without mistaking it for a seek, while staying far below the
 *  scale of any deliberate scrub/skip (typically hundreds of ms to seconds). */
export const NATURAL_TICK_MAX_DELTA_MS = 500;

/** True when advancing from `prevMs` to `nextMs` is consistent with the composite loop's own
 *  natural playback reporting rather than a discontinuous jump (a scrub, a skip button, a loop
 *  back to 0). Used to decide whether a Move-mode PiP drag's unsaved draft should survive a
 *  `timeMs` change (bug-sweep-2 Task 8 review round 1, Important 3/M6): the draft must persist
 *  through ordinary playback ticks - during AND after the drag itself, until the user seeks or
 *  acts - and clear only on an actual discontinuity.
 *
 *  Any change while paused is a deliberate seek by definition (there is no "natural progression"
 *  with nothing driving the clock), so this is unconditionally `false` when `!playing`. While
 *  playing, only a small, FORWARD delta counts as natural - a backward jump (rewind/loop) or one
 *  bigger than `maxDeltaMs` is still treated as a seek even mid-playback (e.g. dragging the
 *  timeline scrubber without pausing first). */
export function isNaturalPlaybackTick(prevMs: number, nextMs: number, playing: boolean, maxDeltaMs: number = NATURAL_TICK_MAX_DELTA_MS): boolean {
  if (!playing) return false;
  const delta = nextMs - prevMs;
  return delta >= 0 && delta <= maxDeltaMs;
}
