import type { EditDoc } from "../edit";
import { outDurMs, outOf, type TimeMap } from "./remap";

export function remapDoc(doc: EditDoc, map: TimeMap): EditDoc {
  const span = (s: number, e: number): [number, number] | null => {
    const [a, b] = [outOf(map, s), outOf(map, e)];
    return b > a ? [a, b] : null;
  };
  const moved = <T extends { start_ms: number; end_ms: number }>(list: T[]): T[] =>
    list.flatMap((r) => {
      const sp = span(r.start_ms, r.end_ms);
      return sp ? [{ ...r, start_ms: sp[0], end_ms: sp[1] }] : [];
    });
  return {
    ...doc,
    zooms: moved(doc.zooms),
    layout: moved(doc.layout),
    effects: moved(doc.effects),
    texts: moved(doc.texts),
    captions: doc.captions.flatMap((c) => {
      const sp = span(c.start_ms, c.end_ms);
      return sp ? [{ ...c, start_ms: sp[0], end_ms: sp[1], words: moved(c.words) }] : [];
    }),
    camera_moves: doc.camera_moves.map((m) => ({ ...m, t_ms: outOf(map, m.t_ms) })),
    trim: { in_ms: 0, out_ms: 0 },
    cuts: [],
    speed: [],
    clips: [],
    clip_ms: outDurMs(map),
  };
}
