# src/editor/hooks/stage/useCompositeLoop.ts

The Stage's single `requestAnimationFrame` loop. Reads the live time, jumps and rate-adjusts the three media elements over the edit's cuts and speed spans, keeps the webcam/audio roughly synced to the screen video, and - on a tick that is actually going to paint - hands the frame to `drawCompositeFrame` (`compositeFrame.ts`), which owns the whole picture. This file is the clock and the media; that one is the paint.

## useCompositeLoop

**Time remap (two clocks per tick).** The loop takes `mapRef` (the `TimeMap` from `useTimeMap`). The media, the clicks, the ripples and the cursor run on clip time `t`; `tOut = outOf(map, t)` is what `camAt`, `layoutAt`, `frameCamLayout`, `resolveSpotlight` and a video background read, because the camera track and every region arrive on the output clock. While playing, `playbackAction` jumps all three media over a cut (and skips that frame's composite, so no frame from inside the cut ever shows) and sets `playbackRate` to the containing speed span's factor; `isCutJump` keeps the discontinuous-jump reset (trail + spotlight sims) from firing on that jump, as the export keeps its state across a cut too. The FX-overlay request block lives in `fxRequestTick.ts`, the parameter types in `compositeLoopRefs.ts`, and everything this loop draws in `compositeFrame.ts`.

```ts
export function useCompositeLoop(refs: CompositeLoopRefs): void
```

### Inputs

All inputs are `RefObject`s (not plain values) so the one long-lived `rAF` loop always reads current state without re-subscribing - `useStageEngine` (`../../stage/useStageEngine.md`) owns the refs (via `useSyncRefs` for the shared ones) and this hook only reads them each tick. See `useStageEngine.md` and `stageProps.md` for what each one holds; `dirtyRef`, `bgRef`, and `spotSimRef` are the engine's own refs (paused-frame dirty flag, the whole background state - still image plus any video/GIF asset, see `stage/canvas/stageBg.ts` - and the spotlight sim state). `spotSimRef` is created in `useStageEngine` rather than here (unlike the sibling `useRef`s under Notes below) specifically so the engine can ALSO reset it from its own `[effects]` effect (`spotlightEffectsKey`, M9) - a case this loop's own reset (Implementation step 2) can't see because it never fires `t`.

- `dragPoseRef: RefObject<CamPose | null>` - set by `Stage`'s Move-mode drag handle while the user is mid-drag; `null` the rest of the time. When non-`null` it takes precedence over `camMoveAt(cameraMovesRef.current, t)` entirely, so the PiP tracks the live pointer position every frame without writing to the backend - the drag only commits a real `camera_moves` keyframe on pointer-up.
- `arrangingRef: RefObject<boolean>` (T34 L3) - whether stage arrange mode owns the stage. The loop reads the draft as `activeCamDraft(dragPoseRef.current, arrangingRef.current)`, which returns `null` while it is true: arrange mode's own live draft lives in the BASE layout rect, which `dragPoseRef` would otherwise outrank. It suppresses, it does not clear - `camDraftRef` is never written, so an unsaved Move drag survives arrange mode. See `frameCam.md`.

- `captionsRef` / `capStyleRef` / `accentRef` - the caption track (`outDoc.captions`, OUTPUT clock), `doc.settings.captions` and `doc.settings.ui.accent`. The same three inputs Rust `FrameRenderer::composite_at` hands `captiondraw::overlay`, so the preview and the export are fed identically. A caption or style edit is a doc change, so it already reaches this loop through `Stage`'s `drawDeps` and repaints a paused frame.

`exactRef` and `editGenRef` come from `useExactFrame` (its own doc): the export's frame for the paused instant, if fetched, and the edit generation its key must carry.

### Returns

`void` - side-effect only; starts the loop on mount, cancels it on unmount.

### Implementation

1. Each tick: if paused and nothing is `dirtyRef`, skip entirely (the idle/interaction-lag fix - an open-but-idle editor shouldn't burn 60fps redrawing a static frame).
2. Read the time (`sv.currentTime` while playing, else `timeRef`). A discontinuous jump since the last tick (`|t - lastFrameTRef| > 200` or `t` went backward - a seek/scrub, not natural playback advance) clears the cursor trail, resets the cursor's motion-tilt filter (`resetTilt`, `../stage/cursor/cursorTilt.ts` - the mirror of Rust `Cursor::reset`, so the cursor never arrives somewhere else still leaning from the gesture before it) AND resets `spotSimRef` (M9) - `resolveSpotlight`'s handoff transitions run in MEDIA time, so a sim left mid-transition across a jump would keep easing toward a driver that's no longer relevant at the new `t`. Throttle the `onTime` report to ~16fps, nudge webcam/audio back in sync if they've drifted. The tilt filter is then stepped with the OUTPUT-clock delta (`tOut - lastOutTRef`, 0 on a jump), never the clip-time one: its positions come off the output-clock track and Rust hands its own filter the exact output frame period, so a clip delta inside a speed span read the lean at 1/factor of the export (bug sweep 2026-09-15).
3. Get the canvas 2D context and call `drawCompositeFrame(ctx, c, sv, refs, scratch, play, t, tOut, dtOut)` - every pixel of the frame, documented in `compositeFrame.md`.

### Notes

- `lastReportRef`, `lastFrameTRef` and `lastOutTRef` are private to this hook: they are the loop's own memory of the last painted tick. The draw's scratch state (the two canvases, the tilt filter and the five FX-request refs) is one `newFrameScratch()` object created once here and handed to every `drawCompositeFrame` call - see `compositeFrame.md`.
- The effect's dep list is empty on purpose (the `eslint-disable` above it): every input is a `RefObject`, so one long-lived loop always reads current state and never re-subscribes. `refs` itself is captured from the first render, which is safe for the same reason.
