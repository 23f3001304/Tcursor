# src/editor/stage/transport/playback.ts

The preview's playback rules over a `TimeMap` (`shared/math/remap.md`): the `<video>` and `<audio>` elements stay on clip time, so cuts and speed spans have to be acted out on them. Pure, so `playback.test.ts` pins every rule on the parity fixture. It also carries the pure "was this `timeMs` change an ordinary playback tick, or a real seek?" predicate, added in bug-sweep-2 Task 8 review round 1 (Important 3/M6) to replace a `draggingRef`-gated first attempt at the same problem - see `CamDragHandle.md`/`Stage.md` for the full context.

## FRAME_MS

```ts
export const FRAME_MS = 1000 / 60;
```

One output frame at 60fps; the tolerance `isCutJump` is measured in.

## PlaybackAction

```ts
export interface PlaybackAction { seekTo: number | null; rate: number }
```

## playbackAction

```ts
export function playbackAction(map: TimeMap, tMs: number): PlaybackAction
```

What the media elements must do at clip time `tMs` while playing, decided on the SEGMENT LIST, not on a clock: if some segment has `clipStart <= tMs < clipEnd` the instant is kept and shown where it is, so `seekTo` is `null`; otherwise `tMs` falls in a cut, before the trim-in, or in source the clip list never shows, and `seekTo` is `nextShown(map, tMs)?.clipStart` - the start of the next kept segment - or `null` past the last one, where playback runs out as it always has.

Deciding on the segment list rather than on the output clock is what fixes the stranded preview. The old rule seeked to `clipOf(map, outOf(map, tMs))`, and `outOf` rounds: a cut whose next segment starts at a fractional `outStart` (2333.33 for a 1.5x span of 2000 ms before it) rounds DOWN to 2333, which `clipOf` resolves in the segment BEFORE the cut and maps back to 3000, the cut's own start - so the seek either did nothing or drove the media back into the cut, tick after tick. A `clipStart` is exact, needs no round trip, and is what `useCompositeLoop` already jumps every media element to (it also skips that frame's composite, so no frame from inside the cut ever shows).

Since a kept instant now short-circuits before any arithmetic, the `FRAME_MS` tolerance that used to absorb the rounding inside a speed span is no longer needed here; `FRAME_MS` stays exported for `isCutJump` and its callers. Traversing a REORDERED clip list - the media sitting past the last source instant an earlier-listed clip shows, where the next segment in output order is backwards in source - is Batch 4's, together with the per-clip decoder; this rule only ever seeks forward in source. `rate` is unchanged, the containing segment's factor (1 outside every span); the loop writes it to `playbackRate` on all three media (Chromium keeps the pitch).

## isCutJump

```ts
export function isCutJump(map: TimeMap, prevMs: number, tMs: number, frameMs = FRAME_MS): boolean
```

True when a jump from `prevMs` to `tMs` is the preview skipping a cut: the tick that triggered the seek saw a clip time within two frames of the cut's start and this tick landed within two frames of its end. The composite loop's "discontinuous jump" reset (trail and spotlight sims) stands down for it, because the export never saw those frames either and keeps its state across them; any other long jump is a seek and resets as before.

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
