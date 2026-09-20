# src/editor/hooks/stage/useMediaPlayback.ts

Owns play/pause, mute, paused-seek and the dissolve pre-seek for the Stage's hidden `<video>`/`<audio>` elements - the DOM-level media control that's independent of the `rAF` compositing loop.

## useMediaPlayback

```ts
export function useMediaPlayback({
  screenRef, screenBRef, webcamRef, audioRef, playing, src, muted, volume, audioSrc,
  timeMs, preseekMs, playRef,
}: {
  screenRef: RefObject<HTMLVideoElement | null>; screenBRef: RefObject<HTMLVideoElement | null>;
  webcamRef: RefObject<HTMLVideoElement | null>; audioRef: RefObject<HTMLAudioElement | null>;
  playing: boolean; src: string; muted: boolean; volume: number; audioSrc: string; timeMs: number;
  preseekMs: number | null; playRef: RefObject<boolean>;
}): void
```

### Inputs

- `screenRef`/`webcamRef`/`audioRef` - the hidden media elements Stage renders.
- `playing` - play/pause the elements track this; `src` is in the effect deps so a raw→proxy source swap re-evaluates play state without restarting playback from 0.
- `muted`/`volume` - applied to the audio element only (video elements are always `muted` in the DOM; the mixed preview audio is the only audible source). `volume` is the transport slider's 0..1 gain, clamped and set as `audioRef.current.volume`.
- `timeMs`, `playRef` - the paused-seek effect only fires when `playRef.current` is false (checked via the ref, not the captured `playing`, so a stale closure can't seek backward mid-play), and only when an element's `currentTime` has drifted more than 40ms from `timeMs`. On the play *transition* the play/pause effect performs its own one-shot seek to `timeMs` (see below), since this paused-seek effect is disabled while playing.
- `screenBRef`, `preseekMs` (Batch 4 T7) - the clip dissolve's second screen element and the SOURCE instant it should be holding, `preseekAt(dissolves, map, tOut)` from the engine. `null` means no boundary is near and the element is left exactly as it is; it is also what every document without a clip transition passes on every render, and in that case the element does not exist either.

### Returns

`void` - four effects, no return value.

### Implementation

1. **Play/pause effect** (deps: `playing`, `src`, the three refs) - on the play transition, first seeks every element whose `currentTime` differs from `timeMs` (read via a ref so scrub ticks don't re-run the effect) by >50ms, *then* `play().catch(() => {})`; on pause, `.pause()` each element. The pre-play seek is what makes Play-from-outside-the-trim actually restart at the trim-in: the paused-seek effect below is disabled while playing, so without it the media would resume from wherever it stopped and creep past the trim-out on every Play.
2. **Mute/volume effect** (deps: `muted`, `volume`, `audioSrc`, `audioRef`) - sets `audioRef.current.muted` and `.volume` (clamped 0..1).
3. **Paused-seek effect** (deps: `timeMs`, `playing`, the three refs, `playRef`) - no-ops while playing; otherwise nudges any element whose `currentTime*1000` has drifted >40ms from `timeMs`.
4. **Dissolve pre-seek effect** (deps: `preseekMs`, `screenBRef`) - parks the second screen element on `preseekMs` and pauses it, skipping the assignment when it is already within 50ms of it. Unlike the three above it runs WHILE PLAYING too, because the frame it is fetching is needed during playback; it never plays the element, so it cannot become a second clock.

   *Why a second early.* A `currentTime` assignment resolves in roughly 30 to 120 ms in WebView2 (spec 6.5, and the reason Task 6's output clock is element-slaved rather than a wall clock), so an element asked for the outgoing tail at the boundary would still be showing a stale frame for the first frames of the dissolve, which is where the blend is most visible. `preseekAt` widens its window by `PRESEEK_LEAD_MS` (1000) at the front for exactly that, and keeps answering the same instant through the whole dissolve, so the effect re-runs harmlessly and the 50ms guard turns every repeat into a no-op. The instant itself is the last output millisecond of the outgoing clip mapped back to source - the same one the export latches. See `../../stage/clips/clipDissolve.md`.
