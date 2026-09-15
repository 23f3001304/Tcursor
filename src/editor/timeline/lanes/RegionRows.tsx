import { memo } from "react";
import { motion } from "motion/react";
import type { Drag } from "../../hooks/input/useRegionDrag";
import { pillLeftPct, pillWidthPct } from "../model/pillGeometry";

interface Region {
  id: string;
  start_ms: number;
  end_ms: number;
  layer: number;
}

function RegionRowsInner<T extends Region>({
  rows,
  regions,
  dur,
  sel,
  rowClass,
  blkClass,
  dragState,
  beginDrag,
  renderLabel,
  extraStyle,
  titleOf,
}: {
  rows: number;
  regions: T[];
  dur: number;
  sel: string | null;
  rowClass: string;
  blkClass: string;
  dragState: Drag | null;
  beginDrag: (ev: React.PointerEvent, id: string, mode: "move" | "l" | "r", s: number, e: number) => void;
  renderLabel: (r: T) => React.ReactNode;
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties;
  titleOf?: (r: T) => string | undefined;
}) {
  return (
    <>
      {Array.from({ length: rows }, (_, i) => rows - 1 - i).map((layer) => (
        <div className={rowClass} key={`${rowClass}${layer}`}>
          {regions
            .filter((r) => r.layer === layer)
            .sort((a, b) => a.start_ms - b.start_ms)
            .map((r, i, row) => {
              const dragging = dragState?.id === r.id;
              const s = dragging && dragState ? dragState.start : r.start_ms;
              const e = dragging && dragState ? dragState.end : r.end_ms;
              const leftPct = pillLeftPct(s, dur);
              const nextStart = row[i + 1]?.start_ms;
              return (
                <motion.div
                  key={r.id}
                  data-region-id={r.id}
                  title={titleOf?.(r)}
                  className={`${blkClass}${sel === r.id ? " sel" : ""}${dragging ? " drag" : ""}`}
                  style={{
                    left: `${leftPct}%`,
                    width: `${pillWidthPct(s, e, dur, leftPct, nextStart)}%`,
                    ...extraStyle?.(r, s, e),
                  }}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1, y: dragging && dragState ? dragState.dyPx : 0, scale: 1 }}
                  transition={{
                    opacity: { type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] },
                    y: dragging ? { duration: 0 } : { type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] },
                  }}
                  onPointerDown={(ev) => beginDrag(ev, r.id, "move", r.start_ms, r.end_ms)}
                >
                  <span
                    className="e-zh"
                    onPointerDown={(ev) => beginDrag(ev, r.id, "l", r.start_ms, r.end_ms)}
                  />
                  <span className="e-zlabel">{renderLabel(r)}</span>
                  <span
                    className="e-zh"
                    onPointerDown={(ev) => beginDrag(ev, r.id, "r", r.start_ms, r.end_ms)}
                  />
                </motion.div>
              );
            })}
        </div>
      ))}
    </>
  );
}

export const RegionRows = memo(RegionRowsInner) as typeof RegionRowsInner;
