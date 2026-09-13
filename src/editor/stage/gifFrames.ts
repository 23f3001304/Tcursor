// GIF decoding for the preview's background.
//
// A GIF is a `video` in the settings model (one export decode path for both), but a `<video>`
// element cannot play one. The WebView here is Chromium (WebView2), so the frames come from the
// WebCodecs `ImageDecoder` API: decode every frame once, keep its cumulative end time, and pick a
// frame per preview tick by the wrapped playhead. That gives a GIF the same scrub-anywhere
// behaviour a video gets from `currentTime`, which an `<img>` (whose animation runs on its own
// wall clock and cannot be seeked) never could.

/** Every decoded frame plus the cumulative end time of each, and the loop length. */
export interface GifFrames { frames: ImageBitmap[]; ends: number[]; totalMs: number }

/** A frame with no stated duration: what browsers themselves show such a GIF frame for. */
const DEFAULT_FRAME_MS = 100;

/** Per-frame durations (ms) as cumulative END times. A zero-length frame is given the default
 *  rather than kept at zero, which would make its slot unreachable and shorten the whole loop. */
export function frameEnds(durations: number[]): number[] {
  let acc = 0;
  return durations.map((d) => {
    acc += d > 0 ? d : DEFAULT_FRAME_MS;
    return acc;
  });
}

/** The frame showing at `tMs`, wrapped into the loop. 0 for an empty list (so a draw before the
 *  decode lands is never out of range) and 0 for a playhead before the clip starts - the same
 *  answer `stageBg`'s `loopMs` gives a video, so the two paths agree at the edges. */
export function gifIndexAt(ends: number[], tMs: number): number {
  const total = ends[ends.length - 1] ?? 0;
  if (!ends.length || total <= 0 || !(tMs > 0)) return 0;
  const t = tMs % total;
  const i = ends.findIndex((e) => t < e);
  return i < 0 ? 0 : i;
}

/** Decode every frame of the GIF at `url`. `null` when `ImageDecoder` is unavailable or the file
 *  cannot be read - the caller then falls back to a plain `<img>`, i.e. the GIF's first frame. */
export async function decodeGif(url: string): Promise<GifFrames | null> {
  const Decoder = (globalThis as { ImageDecoder?: typeof ImageDecoder }).ImageDecoder;
  if (!Decoder || !url) return null;
  try {
    const data = await fetch(url).then((r) => r.arrayBuffer());
    const dec = new Decoder({ data, type: "image/gif" });
    await dec.completed;
    const count = dec.tracks.selectedTrack?.frameCount ?? 0;
    const frames: ImageBitmap[] = [];
    const durations: number[] = [];
    for (let i = 0; i < count; i++) {
      const { image } = await dec.decode({ frameIndex: i });
      // `duration` is microseconds (VideoFrame's unit), and may be null on a malformed frame.
      durations.push((image.duration ?? 0) / 1000);
      frames.push(await createImageBitmap(image));
      image.close();
    }
    dec.close();
    if (!frames.length) return null;
    const ends = frameEnds(durations);
    return { frames, ends, totalMs: ends[ends.length - 1] };
  } catch {
    return null; // a corrupt or unreadable GIF is a still background, not a broken editor
  }
}

/** Release the decoded bitmaps. Called when the asset changes or the stage unmounts: each frame
 *  is a full-size GPU-backed bitmap, and a long GIF holds a lot of them. */
export function closeGif(g: GifFrames | null) {
  g?.frames.forEach((f) => f.close());
}
