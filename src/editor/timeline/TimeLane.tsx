import { memo, useCallback, useMemo } from "react";
import type { EditDoc, EditOp, Speed } from "../../lib/edit";
import { useRegionDrag } from "../hooks/useRegionDrag";
import { RegionRows } from "./RegionRows";

/** The Time lane: one pill per speed span, on the same plane every other lane's pills use, labelled
 *  with the factor it applies. Cuts share this lane's time but not its row - they are drawn across
 *  the whole track stack by `CutOverlay`, because a cut removes time from every lane at once.
 *
 *  Speed spans are kept disjoint by the ops (`normalize_speed`), so they never need more than one
 *  row and `layer` is a constant 0 - the row-stacking `useRegionDrag` reports is ignored on commit,
 *  which is why `update_speed` carries no layer. */

/** `2x`, `0.5x`: the factor with no trailing zeros. Module scope (not an inline arrow) so
 *  `RegionRows`' memo survives a playhead tick - the convention `Timeline.md` records. */
const speedLabel = (s: Speed) => `${+s.factor.toFixed(2)}x`;

export const TimeLane = memo(function TimeLane({ doc, dur, sel, onSel, onApply, track }: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}) {
  // Memoized on the doc's own array so `beginDrag` and `RegionRows`' memo stay stable across a tick.
  const spans = useMemo(() => doc.speed.map((s) => ({ ...s, layer: 0 })), [doc.speed]);
  const onCommit = useCallback((id: string, start_ms: number, end_ms: number) =>
    void onApply({ op: "update_speed", id, start_ms, end_ms }), [onApply]);
  const { drag, beginDrag } = useRegionDrag(spans, dur, track, 32, onCommit, onSel);
  return (
    <RegionRows rows={1} regions={spans} dur={dur} sel={sel} rowClass="e-timerow" blkClass="e-spdblk"
      dragState={drag} beginDrag={beginDrag} renderLabel={speedLabel} />
  );
});
