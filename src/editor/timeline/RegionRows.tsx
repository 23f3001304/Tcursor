import { memo } from "react";
import { motion } from "motion/react";
import type { Drag } from "../hooks/useRegionDrag";
import { pillLeftPct, pillWidthPct } from "./pillGeometry";

interface Region { id: string; start_ms: number; end_ms: number; layer: number }

/** Shared row/pill renderer for the zoom, FX, and layout lanes (Timeline.tsx) - identical
 *  layer-stacked rows, drag/hover motion, and grip-handle markup; the three lanes differ only in
 *  fill color (`blkClass`, a CSS concern), the pill's label content, and (layout only) the inline
 *  fade-ramp CSS vars, all passed in rather than triplicated here. Extracted so adding the lane
 *  label gutter (Task 36) didn't push Timeline.tsx over the file's line budget.
 *
 *  `React.memo`'d (render hygiene pass) - `Timeline` passes stable/memoized `regions`,
 *  `dragState`, `beginDrag` and (module-level, not inline) `renderLabel`/`extraStyle`, so this
 *  only re-renders when the lane's own data actually changes, not on every playhead tick. */
function RegionRowsInner<T extends Region>({ rows, regions, dur, sel, rowClass, blkClass, dragState, beginDrag, renderLabel, extraStyle }: {
  rows: number; regions: T[]; dur: number; sel: string | null;
  rowClass: string; blkClass: string;
  dragState: Drag | null;
  beginDrag: (ev: React.PointerEvent, id: string, mode: "move" | "l" | "r", s: number, e: number) => void;
  renderLabel: (r: T) => React.ReactNode;
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties;
}) {
  // The dragged pill stays in its ORIGINAL row's DOM the whole drag (grouped by the static
  // region.layer, not the live snapped target) and instead visually follows the pointer via its
  // own `y` motion value at zero-duration (instant, no spring lag) - moving it into a different
  // row's <div> live would remount it (a different React parent), replaying its mount fade-in
  // every time it crossed a row boundary. It only actually reflows into the new row once the
  // drag ends and `regions` re-renders with the committed layer.
  return (
    <>
      {Array.from({ length: rows }, (_, i) => rows - 1 - i).map((layer) => (
        <div className={rowClass} key={`${rowClass}${layer}`}>
          {regions.filter((r) => r.layer === layer).map((r) => {
            const dragging = dragState?.id === r.id;
            const s = dragging && dragState ? dragState.start : r.start_ms;
            const e = dragging && dragState ? dragState.end : r.end_ms;
            const leftPct = pillLeftPct(s, dur);
            return (
              <motion.div key={r.id} data-region-id={r.id} className={`${blkClass}${sel === r.id ? " sel" : ""}${dragging ? " drag" : ""}`}
                style={{ left: `${leftPct}%`, width: `${pillWidthPct(s, e, dur, leftPct)}%`, ...extraStyle?.(r, s, e) }}
                initial={{ opacity: 0 }} whileHover={{ scale: 1.02, transition: { duration: 0.12 } }}
                animate={{ opacity: 1, y: dragging && dragState ? dragState.dyPx : 0, scale: 1 }}
                transition={{ opacity: { type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] },
                  y: dragging ? { duration: 0 } : { type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] } }}
                onPointerDown={(ev) => beginDrag(ev, r.id, "move", r.start_ms, r.end_ms)}>
                <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, r.id, "l", r.start_ms, r.end_ms)} />
                <span className="e-zlabel">{renderLabel(r)}</span>
                <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, r.id, "r", r.start_ms, r.end_ms)} />
              </motion.div>
            );
          })}
        </div>
      ))}
    </>
  );
}

// `memo` doesn't preserve a generic function's type parameters on its own - the cast restores
// `RegionRows<T>`'s real (generic) call signature for every call site.
export const RegionRows = memo(RegionRowsInner) as typeof RegionRowsInner;
