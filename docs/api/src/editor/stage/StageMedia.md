# src/editor/stage/StageMedia.tsx

The Stage's hidden native media elements: the screen and webcam `<video>`s that `useCompositeLoop` reads every frame via `canvas.drawImage` (never actually shown on screen - see `HIDDEN`), a SECOND screen `<video>` for the clip dissolve, the mixed preview `<audio>`, and (on `err`) a recoverable error card with a Retry button. Holds no state of its own - `Stage` owns the refs, the `err` string, and every event-handler closure; this component only renders what it's handed.

## MEDIA_ERR

```ts
export const MEDIA_ERR: string[]
```

`HTMLMediaElement.error.code` (1..4) to a human-readable reason, indexed directly; index 0 is the unused `""` slot so the code IS the index. `Stage`'s `onScreenError` handler maps through it and falls back to `"load failed"`.

## StageMedia

```tsx
export function StageMedia({
  screenRef, screenBRef, wantsScreenB, webcamRef, audioRef, src, webcamSrc, audioSrc, err, onRetry,
  onScreenLoadedData, onScreenSeeked, onScreenEnded, onScreenLoadedMetadata, onScreenError,
  onWebcamLoadedData, onWebcamSeeked,
}): JSX.Element
```

Renders (in order) the screen `<video>` when `src` is set, the dissolve's second screen `<video>` when `src` is set and `wantsScreenB`, the webcam `<video>` when `webcamSrc` is set, the preview `<audio>` when `audioSrc` is set, and the error card when `err` is non-null.

### Props

- `screenRef` / `webcamRef` / `audioRef` (`RefObject<HTMLVideoElement | null>` / `RefObject<HTMLVideoElement | null>` / `RefObject<HTMLAudioElement | null>`) - the same ref objects `Stage` also hands to `useMediaPlayback` and `useCompositeLoop`, so the elements this component renders are the exact ones those hooks read/control. *Why refs are props here:* `Stage` must create them (its hooks need them before this component even mounts, e.g. when `src` is empty), so ownership stays in `Stage` and only the JSX moves.
- `screenBRef: RefObject<HTMLVideoElement | null>` / `wantsScreenB: boolean` (Batch 4 T7) - the clip dissolve's second screen element and the one condition it exists under. It carries the SAME `src` as the first and is never played: `useMediaPlayback`'s pre-seek effect parks it on the outgoing clip's tail a second before a boundary and pauses it, and `drawScreenPanel` draws that frame under the incoming one for the length of the transition. Its one handler is `onScreenSeeked`, the same callback the first screen element carries, because the semantics are the same: a `currentTime` assignment resolves asynchronously, and a click landing straight inside a dissolve window composites before the pre-seek lands. Without it nothing marked the canvas dirty when the frame arrived and the stale picture stood until the exact frame did, up to `SETTLE_MS` later. *Why it is conditional where the others are conditional on their URL:* a second decoder on the same file is not free, and `wantsScreenB` is `dissolves.length > 0` from the engine (`useStageEngine.md`), which is false for every document that has no clips, one clip, or clips whose transitions are all zero - so a project that cannot dissolve mounts nothing at all. A lone clip carrying a leftover transition counts as cannot: nothing precedes it, so no window resolves (ruling, `clips/clipDissolve.md`).
- `src: string` / `webcamSrc: string` / `audioSrc: string` - the three media URLs; each element only renders when its URL is truthy (a falsy `src` skips the `<video>` entirely rather than rendering one with an empty `src` attribute).
- `err: string | null` - the current load-error message (or `null`). Drives the `.e-media-err` error card.
- `onRetry: () => void` - the error card's Retry button. `Stage` supplies a handler that clears `err`, calls `useEditorData`'s `retryMedia`, and force-reloads the screen `<video>` (see `Stage.md`'s Behavior section for why all three).
- `onScreenLoadedData: () => void` / `onScreenSeeked: () => void` - fired on the screen video's `loadeddata`/`seeked`; `Stage` uses these to clear `err` and mark the paused-idle composite loop dirty. `onScreenSeeked` is wired to BOTH screen elements, so a pre-seek landing on the dissolve's second video repaints the same way a scrub on the first one does.
- `onScreenEnded: () => void` - fired on the screen video's `ended`; `Stage` reports the exact rounded duration as the final playhead position (the throttled `onTime` can otherwise miss the last tick).
- `onScreenLoadedMetadata: (e: SyntheticEvent<HTMLVideoElement>) => void` - fired on the screen video's `loadedmetadata`; `Stage` reports the true duration, restores `currentTime`, and resumes playback if it was playing - handles the raw-capture-to-proxy `src` swap without resetting the playhead.
- `onScreenError: (e: SyntheticEvent<HTMLVideoElement>) => void` - fired on the screen video's `error`; `Stage` maps `e.currentTarget.error?.code` through `MEDIA_ERR` into a human-readable string and sets `err`.
- `onWebcamLoadedData: () => void` / `onWebcamSeeked: () => void` - fired on the webcam video's `loadeddata`/`seeked`; `Stage` marks the composite loop dirty (a paused webcam seek must still repaint since it's composited every frame).

### Behaviors

`StageMedia.test.tsx`, jsdom, `createRoot`/`act` and a dispatched `seeked` Event (React attaches media events straight to the element rather than delegating them, so the dispatch reaches the prop).

- `mounts the dissolve's second screen video only when the document can dissolve` - one `<video>` with `wantsScreenB` false, two with it true.
- `repaints when the second video's pre-seek lands, exactly as the first one does` - a `seeked` on either screen element calls `onScreenSeeked`.

### Notes

- `HIDDEN` (an inline `position: absolute; width: 1; height: 1; opacity: 0; pointerEvents: none` style) keeps both `<video>`s off-screen but still decoding.
- The component returns a Fragment, not a wrapping element, so it adds no DOM node between `.e-stage` and these children.
