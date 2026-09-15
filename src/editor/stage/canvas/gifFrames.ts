export interface GifFrames {
  frames: ImageBitmap[];
  ends: number[];
  totalMs: number;
}

const DEFAULT_FRAME_MS = 100;

export function frameEnds(durations: number[]): number[] {
  let acc = 0;
  return durations.map((d) => {
    acc += d > 0 ? d : DEFAULT_FRAME_MS;
    return acc;
  });
}

export function gifIndexAt(ends: number[], tMs: number): number {
  const total = ends[ends.length - 1] ?? 0;
  if (!ends.length || total <= 0 || !(tMs > 0)) return 0;
  const t = tMs % total;
  const i = ends.findIndex((e) => t < e);
  return i < 0 ? 0 : i;
}

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
      durations.push((image.duration ?? 0) / 1000);
      frames.push(await createImageBitmap(image));
      image.close();
    }
    dec.close();
    if (!frames.length) return null;
    const ends = frameEnds(durations);
    return { frames, ends, totalMs: ends[ends.length - 1] };
  } catch {
    return null;
  }
}

export function closeGif(g: GifFrames | null) {
  g?.frames.forEach((f) => f.close());
}
