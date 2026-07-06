# src/editor/hooks/useMediaPlayback.ts

Owns play/pause, mute, and paused-seek for the Stage's hidden `<video>`/`<audio>` elements - the DOM-level media control that's independent of the `rAF` compositing loop.

## useMediaPlayback

```ts
export function useMediaPlayback({
  screenRef, webcamRef, audioRef, playing, src, muted, audioSrc, timeMs, playRef,
}: {
  screenRef: RefObject<HTMLVideoElement | null>; webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>;
  playing: boolean; src: string; muted: boolean; audioSrc: string; timeMs: number;
  playRef: RefObject<boolean>;
}): void
```

### Inputs

- `screenRef`/`webcamRef`/`audioRef` - the hidden media elements Stage renders.
- `playing` - play/pause the elements track this; `src` is in the effect deps so a raw→proxy source swap re-evaluates play state without restarting playback from 0.
- `muted` - applied to the audio element only (video elements are always `muted` in the DOM; the mixed preview audio is the only audible source).
- `timeMs`, `playRef` - the paused-seek effect only fires `playRef.current` is false (checked via the ref, not the captured `playing`, so a stale closure can't seek backward mid-play), and only when an element's `currentTime` has drifted more than 40ms from `timeMs`.

### Returns

`void` - three effects, no return value.

### Implementation

1. **Play/pause effect** (deps: `playing`, `src`, the three refs) - `play().catch(() => {})` or `.pause()` each element that exists.
2. **Mute effect** (deps: `muted`, `audioSrc`, `audioRef`) - sets `audioRef.current.muted`.
3. **Paused-seek effect** (deps: `timeMs`, `playing`, the three refs, `playRef`) - no-ops while playing; otherwise nudges any element whose `currentTime*1000` has drifted >40ms from `timeMs`.
