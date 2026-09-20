import { clipOf, outDurMs, type Segment, type TimeMap } from "../../../shared/math/remap";
import { FRAME_MS, NATURAL_TICK_MAX_DELTA_MS } from "./playback";

export interface ClipTick {
  tOut: number;
  t: number;
  seekTo: number | null;
  ended: boolean;
}

const outLen = (s: Segment) => (s.clipEnd - s.clipStart) / s.factor;

function segAtOut(map: TimeMap, outMs: number): number {
  const i = map.segments.findIndex((s) => outMs < s.outStart + outLen(s));
  return i < 0 ? map.segments.length - 1 : i;
}

export function needsOutClock(map: TimeMap): boolean {
  const first = map.segments[0];
  return first !== undefined && map.segments.some((s) => s.clip !== first.clip);
}

export function clipTick(map: TimeMap, outMs: number, mediaMs: number, frameMs = FRAME_MS): ClipTick {
  const segs = map.segments;
  const end = outDurMs(map);
  if (segs.length === 0) return { tOut: 0, t: map.trimIn, seekTo: null, ended: true };
  const o = Math.min(Math.max(0, outMs), end);
  let i = segAtOut(map, o);
  const from = segs[i];

  if (!(mediaMs >= from.clipStart - frameMs && mediaMs < from.clipEnd)) {
    const ranOff = mediaMs >= from.clipEnd && mediaMs - from.clipEnd <= NATURAL_TICK_MAX_DELTA_MS;
    if (!ranOff) {
      const stray = clipOf(map, o);
      return { tOut: o, t: stray, seekTo: stray, ended: false };
    }
    while (i + 1 < segs.length && segs[i + 1].clipStart === segs[i].clipEnd && mediaMs >= segs[i + 1].clipEnd)
      i++;
    const next = segs[i + 1];
    if (!next) {
      const last = segs[i];
      return { tOut: end, t: Math.max(last.clipStart, last.clipEnd - 1), seekTo: null, ended: true };
    }
    if (next.clipStart !== segs[i].clipEnd)
      return { tOut: next.outStart, t: next.clipStart, seekTo: next.clipStart, ended: false };
    i++;
  }

  const s = segs[i];
  const t = Math.max(mediaMs, s.clipStart);
  return { tOut: s.outStart + (t - s.clipStart) / s.factor, t, seekTo: null, ended: false };
}
