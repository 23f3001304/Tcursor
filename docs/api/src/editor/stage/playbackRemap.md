# src/editor/stage/playbackRemap.ts

The preview's playback rules over a `TimeMap` (`lib/remap.md`): the `<video>` and `<audio>` elements stay on clip time, so cuts and speed spans have to be acted out on them. Pure, so `playbackRemap.test.ts` pins every rule on the parity fixture.

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

What the media elements must do at clip time `tMs` while playing. Inside a finite gap (a cut, or before the trim-in) `seekTo` is the gap's end: `useCompositeLoop` jumps every media element there and skips that frame's composite, so no frame from inside the cut ever shows. The trailing gap past the trim-out has no end to jump to, so playback runs out as it always has. `rate` is the containing segment's factor (1 outside every span); the loop writes it to `playbackRate` on all three media (Chromium keeps the pitch).

## isCutJump

```ts
export function isCutJump(map: TimeMap, prevMs: number, tMs: number, frameMs = FRAME_MS): boolean
```

True when a jump from `prevMs` to `tMs` is the preview skipping a cut: the tick that triggered the seek saw a clip time within two frames of the cut's start and this tick landed within two frames of its end. The composite loop's "discontinuous jump" reset (trail and spotlight sims) stands down for it, because the export never saw those frames either and keeps its state across them; any other long jump is a seek and resets as before.
