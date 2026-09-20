import { memo, useCallback, useMemo } from "react";
import type { Clip, EditDoc, EditOp } from "../../../shared/edit";
import type { TimeMap } from "../../../shared/math/remap";
import { useClipDrag } from "../../hooks/input/useClipDrag";
import { clipExtraStyle, clipLabel, clipRegions, dropIndex } from "../model/clipModel";
import { RegionRows } from "./RegionRows";

export const ClipLane = memo(function ClipLane({
  clips,
  map,
  dur,
  sel,
  onSel,
  onApply,
  track,
}: {
  clips: Clip[];
  map: TimeMap;
  dur: number;
  sel: string | null;
  onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  track: React.RefObject<HTMLDivElement | null>;
}) {
  const regions = useMemo(() => clipRegions(clips, map), [clips, map]);
  const onReorder = useCallback(
    (id: string, atMs: number) => {
      const to = dropIndex(clips, id, atMs);
      if (to !== null) void onApply({ op: "move_clip", id, to_index: to });
    },
    [clips, onApply],
  );
  const onRetime = useCallback(
    (id: string, srcIn: number, srcOut: number) => {
      const c = clips.find((x) => x.id === id);
      if (c && c.src_in_ms === srcIn && c.src_out_ms === srcOut) return;
      void onApply({ op: "update_clip", id, src_in_ms: srcIn, src_out_ms: srcOut });
    },
    [clips, onApply],
  );
  const { drag, beginDrag } = useClipDrag(regions, dur, track, onReorder, onRetime, onSel);
  return (
    <RegionRows
      rows={Math.max(1, ...regions.map((r) => r.layer + 1))}
      regions={regions}
      dur={dur}
      sel={sel}
      rowClass="e-cliprow"
      blkClass="e-clipblk"
      dragState={drag}
      beginDrag={beginDrag}
      renderLabel={clipLabel}
      extraStyle={clipExtraStyle}
      titleOf={(c) =>
        `Clip ${c.order}. Drag it onto another clip to change the export order, drag its edges to retime it.`
      }
    />
  );
});
