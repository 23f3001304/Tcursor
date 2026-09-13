import { useMemo } from "react";
import type { EditDoc } from "../../lib/edit";
import { buildTimeMap, identityMap, type TimeMap } from "../../lib/remap";
import { remapDoc } from "../../lib/remapDoc";

/** The clip-to-output clock map for the doc, and the doc's regions moved onto the output clock,
 *  recomputed once per doc change and never per tick. `outDoc` is what the stage evaluates (zooms,
 *  layouts, effects, camera moves in the time the viewer sees); the timeline keeps `doc` itself,
 *  whose pills sit on clip time and never move when a cut is added. `fullDurMs` is the raw clip's
 *  length. With no doc yet: an identity map and no `outDoc`. */
export function useTimeMap(doc: EditDoc | null, fullDurMs: number): { map: TimeMap; outDoc: EditDoc | null } {
  return useMemo(() => {
    if (!doc) return { map: identityMap(fullDurMs), outDoc: null };
    const map = buildTimeMap(doc.trim, doc.cuts, doc.speed, fullDurMs);
    return { map, outDoc: remapDoc(doc, map) };
  }, [doc, fullDurMs]);
}
