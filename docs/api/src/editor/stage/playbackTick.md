# src/editor/stage/playbackTick.ts

Pure "was this `timeMs` change an ordinary playback tick, or a real seek?" predicate, added in bug-sweep-2 Task 8 review round 1 (Important 3/M6) to replace a `draggingRef`-gated first attempt at the same problem - see `CamDragHandle.md`/`Stage.md` for the full context.

## NATURAL_TICK_MAX_DELTA_MS

```ts
export const NATURAL_TICK_MAX_DELTA_MS = 500
```

Default slack (ms) above the composite loop's own throttled report cadence (`useCompositeLoop.ts`'s `t - lastReportRef.current >= 60`) - generous enough to absorb a janky or backgrounded-tab frame without mistaking it for a seek, while staying far below the scale of any deliberate scrub/skip (typically hundreds of ms to seconds).

## isNaturalPlaybackTick

```ts
export function isNaturalPlaybackTick(prevMs: number, nextMs: number, playing: boolean, maxDeltaMs: number = NATURAL_TICK_MAX_DELTA_MS): boolean
```

### Inputs

- `prevMs` / `nextMs` - the `timeMs` value before and after the change being evaluated.
- `playing: boolean` - whether playback is currently running.
- `maxDeltaMs` - the forward-delta ceiling; defaults to `NATURAL_TICK_MAX_DELTA_MS`.

### Returns

`false` unconditionally when `!playing` - there is no "natural progression" to compare against with nothing driving the clock, so ANY change while paused is by definition a deliberate seek. While playing, `true` only for a small, FORWARD delta (`0 <= nextMs - prevMs <= maxDeltaMs`) - a backward jump (rewind, loop back to 0) or one bigger than `maxDeltaMs` is still treated as a seek even mid-playback (e.g. dragging the timeline scrubber without pausing first).

### Used by

`Stage.tsx`'s and `CamDragHandle.tsx`'s `[timeMs]` effects - both gate their Move-mode draft reset (`camDraftRef`/local `dragPose` respectively) on this, so the unsaved drag pose survives ordinary playback ticks, during AND after the drag itself, and clears only on an actual discontinuity. The OTHER clear trigger - an explicit action (`CameraPanel.tsx`'s `addKeyframeHere`) consuming the draft - doesn't go through this function at all; see each file's own doc.
