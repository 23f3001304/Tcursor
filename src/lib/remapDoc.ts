import type { EditDoc } from "./edit";
import { outOf, type TimeMap } from "./remap";

/** TS mirror of Rust `edit::remap_doc`: every region list moved onto the OUTPUT clock so the preview
 *  evaluates zooms, layouts, effects and camera moves in the time the viewer sees. Durations
 *  (`zoom_in_ms`, `transition_ms`, fades) are NOT scaled, on purpose; a region that collapses inside
 *  a cut is dropped; ids are kept; trim, cuts and speed come back cleared (consumed). Recomputed once
 *  per doc change (`useTimeMap`), never per tick. The Rust side also rewrites `clip_ms`, which the TS
 *  `EditDoc` does not carry (a pre-existing mirror gap). */
export function remapDoc(doc: EditDoc, map: TimeMap): EditDoc {
  const span = (s: number, e: number): [number, number] | null => {
    const [a, b] = [outOf(map, s), outOf(map, e)];
    return b > a ? [a, b] : null;
  };
  const moved = <T extends { start_ms: number; end_ms: number }>(list: T[]): T[] =>
    list.flatMap((r) => { const sp = span(r.start_ms, r.end_ms); return sp ? [{ ...r, start_ms: sp[0], end_ms: sp[1] }] : []; });
  return {
    ...doc,
    zooms: moved(doc.zooms),
    layout: moved(doc.layout),
    effects: moved(doc.effects),
    camera_moves: doc.camera_moves.map((m) => ({ ...m, t_ms: outOf(map, m.t_ms) })),
    trim: { in_ms: 0, out_ms: 0 },
    cuts: [],
    speed: [],
  };
}
