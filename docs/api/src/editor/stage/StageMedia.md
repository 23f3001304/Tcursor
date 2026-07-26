# src/editor/stage/StageMedia.tsx

The Stage's hidden native media elements: the screen and webcam `<video>`s that `useCompositeLoop` reads every frame via `canvas.drawImage` (never actually shown on screen - see `HIDDEN`), the mixed preview `<audio>`, and the "preview unavailable" error overlay. Extracted from `Stage.tsx` to keep that file under the 200-line limit. Holds no state of its own - `Stage` owns the refs, the `err` string, and every event-handler closure; this component only renders what it's handed, so its output is byte-identical to the markup that used to live inline in `Stage.tsx`.

## StageMedia

```tsx
export function StageMedia({
  screenRef, webcamRef, audioRef, src, webcamSrc, audioSrc, err,
  onScreenLoadedData, onScreenSeeked, onScreenEnded, onScreenLoadedMetadata, onScreenError,
  onWebcamLoadedData, onWebcamSeeked,
}): JSX.Element
```

Renders (in order) the screen `<video>` when `src` is set, the webcam `<video>` when `webcamSrc` is set, the preview `<audio>` when `audioSrc` is set, and the error overlay when `err` is non-null - the same conditional structure and DOM order `Stage.tsx` rendered inline before this extraction.

### Props

- `screenRef` / `webcamRef` / `audioRef` (`RefObject<HTMLVideoElement | null>` / `RefObject<HTMLVideoElement | null>` / `RefObject<HTMLAudioElement | null>`) - the same ref objects `Stage` also hands to `useMediaPlayback` and `useCompositeLoop`, so the elements this component renders are the exact ones those hooks read/control. *Why refs are props here:* `Stage` must create them (its hooks need them before this component even mounts, e.g. when `src` is empty), so ownership stays in `Stage` and only the JSX moves.
- `src: string` / `webcamSrc: string` / `audioSrc: string` - the three media URLs; each element only renders when its URL is truthy (a falsy `src` skips the `<video>` entirely rather than rendering one with an empty `src` attribute).
- `err: string | null` - the current load-error message (or `null`). Drives the `.e-stage-empty` "Preview unavailable - {err}" overlay.
- `onScreenLoadedData: () => void` / `onScreenSeeked: () => void` - fired on the screen video's `loadeddata`/`seeked`; `Stage` uses these to clear `err` and mark the paused-idle composite loop dirty.
- `onScreenEnded: () => void` - fired on the screen video's `ended`; `Stage` reports the exact rounded duration as the final playhead position (the throttled `onTime` can otherwise miss the last tick).
- `onScreenLoadedMetadata: (e: SyntheticEvent<HTMLVideoElement>) => void` - fired on the screen video's `loadedmetadata`; `Stage` reports the true duration, restores `currentTime`, and resumes playback if it was playing - handles the raw-capture-to-proxy `src` swap without resetting the playhead.
- `onScreenError: (e: SyntheticEvent<HTMLVideoElement>) => void` - fired on the screen video's `error`; `Stage` maps `e.currentTarget.error?.code` through `MEDIA_ERR` into a human-readable string and sets `err`.
- `onWebcamLoadedData: () => void` / `onWebcamSeeked: () => void` - fired on the webcam video's `loadeddata`/`seeked`; `Stage` marks the composite loop dirty (a paused webcam seek must still repaint since it's composited every frame).

### Notes

- `HIDDEN` (an inline `position: absolute; width: 1; height: 1; opacity: 0; pointerEvents: none` style) keeps both `<video>`s off-screen but still decoding - moved here from `Stage.tsx` since it's now only used by this file.
- The component returns a Fragment, not a wrapping element, so extracting it does not add any DOM node between `.e-stage` and these children.

### Used by

- `src/editor/stage/Stage.tsx` - renders one `<StageMedia>`, passing the three refs it created via `useRef` plus its local `err` state and the load/seek/error handler closures.
