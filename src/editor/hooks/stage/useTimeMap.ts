import { useMemo } from "react";
import type { EditDoc } from "../../../shared/edit";
import { buildTimeMap, identityMap, type TimeMap } from "../../../shared/math/remap";
import { remapDoc } from "../../../shared/math/remapDoc";

export function useTimeMap(doc: EditDoc | null, fullDurMs: number): { map: TimeMap; outDoc: EditDoc | null } {
  return useMemo(() => {
    if (!doc) return { map: identityMap(fullDurMs), outDoc: null };
    const map = buildTimeMap(doc.trim, doc.cuts, doc.speed, fullDurMs);
    return { map, outDoc: remapDoc(doc, map) };
  }, [doc, fullDurMs]);
}
