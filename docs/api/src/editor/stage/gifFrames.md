# src/editor/stage/gifFrames.ts

GIF decoding for the preview's background.

A GIF is a `video` in the settings model (one export decode path for both), but a `<video>` element cannot play one. The WebView here is Chromium (WebView2), so the frames come from the WebCodecs `ImageDecoder` API: decode every frame once, keep its cumulative end time, and pick a frame per preview tick by the wrapped playhead.

*Why not an `<img>`:* an animated `<img>` runs on its own wall clock. It cannot be seeked, cannot be paused with the transport, and would drift away from the playhead the moment the user scrubbed - so a GIF background would be the one thing on the stage that ignored the timeline. Decoding gives it the same scrub-anywhere behaviour a video gets from `currentTime`.

## GifFrames

```ts
export interface GifFrames { frames: ImageBitmap[]; ends: number[]; totalMs: number }
```

Every decoded frame, the cumulative END time of each, and the loop length.

## DEFAULT_FRAME_MS

```ts
const DEFAULT_FRAME_MS = 100
```

What a frame with no stated duration is shown for - the same fallback browsers themselves use for such a GIF.

## frameEnds

```ts
export function frameEnds(durations: number[]): number[]
```

Per-frame durations (ms) as cumulative end times. A zero-length frame is given `DEFAULT_FRAME_MS` rather than kept at zero, which would make its slot unreachable and shorten the whole loop.

## gifIndexAt

```ts
export function gifIndexAt(ends: number[], tMs: number): number
```

The frame showing at `tMs`, wrapped into the loop. `0` for an empty list, so a draw that happens before the decode lands is never out of range, and `0` for a playhead before the clip starts - the same answer `stageBg`'s `loopMs` gives a video.

## decodeGif

```ts
export async function decodeGif(url: string): Promise<GifFrames | null>
```

Decode every frame of the GIF at `url` into `ImageBitmap`s.

`null` when `ImageDecoder` is unavailable (a non-Chromium host, or an older WebView2) or the file cannot be read or parsed - the caller then shows the still background instead, which for a GIF is its first frame, because that is what `ffio::decode_file_cover` put in the backend's PNG. A corrupt GIF is a still background, never a broken editor.

`VideoFrame.duration` is MICROSECONDS, hence the `/ 1000`; it can also be null on a malformed frame, which `frameEnds` then covers.

## closeGif

```ts
export function closeGif(g: GifFrames | null): void
```

Release the decoded bitmaps. Each is a full-size GPU-backed image and a long GIF holds a lot of them, so this runs whenever the asset changes and when the stage unmounts.

### Behaviors

- `frameEnds` accumulates durations, is empty for no frames, and substitutes the default for a zero-length frame.
- `gifIndexAt` picks the frame whose span contains the time (boundaries included), wraps past the end, clamps before the start, and is 0 for an empty list.

### Used by

- `src/editor/stage/useStageInvalidation.ts` - decodes on an asset change, closes on teardown.
- `src/editor/stage/stageBg.ts` (`drawMoving`) - picks and draws the frame.
