import { memo, useCallback, useMemo } from "react";
import type { EditDoc, EditOp, Speed } from "../../../shared/edit";
import { useRegionDrag } from "../../hooks/input/useRegionDrag";
import { RegionRows } from "./RegionRows";

const speedLabel = (s: Speed) => `${+s.factor.toFixed(2)}x`;

export const TimeLane = memo(function TimeLane({
  doc,
  dur,
  sel,
  onSel,
  onApply,
  track,
}: {
  doc: EditDoc;
  dur: number;
  sel: string | null;
  onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  track: React.RefObject<HTMLDivElement | null>;
}) {
  const spans = useMemo(() => doc.speed.map((s) => ({ ...s, layer: 0 })), [doc.speed]);
  const onCommit = useCallback(
    (id: string, start_ms: number, end_ms: number) =>
      void onApply({ op: "update_speed", id, start_ms, end_ms }),
    [onApply],
  );
  const { drag, beginDrag } = useRegionDrag(spans, dur, track, 32, onCommit, onSel);
  return (
    <RegionRows
      rows={1}
      regions={spans}
      dur={dur}
      sel={sel}
      rowClass="e-timerow"
      blkClass="e-spdblk"
      dragState={drag}
      beginDrag={beginDrag}
      renderLabel={speedLabel}
    />
  );
});
