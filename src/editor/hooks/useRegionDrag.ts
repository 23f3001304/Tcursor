import { useCallback, useEffect, useRef, useState } from "react";
import { pastDragThreshold } from "./dragThreshold";

export type Mode = "move" | "l" | "r";
export interface Drag { id: string; mode: Mode; oStart: number; oEnd: number; start: number; end: number; lo: number; hi: number; layer: number; dyPx: number }
interface Region { id: string; start_ms: number; end_ms: number; layer?: number }

/** Shared drag/resize logic for timeline region pills. Body = move (both horizontally
 *  to retime, and vertically to change priority layer), side handles = resize; a drag
 *  updates a local draft (`drag`) and calls `onCommit` on release.
 *
 *  `dyPx` is the raw, unsnapped vertical pixel offset since the drag began - the caller
 *  applies it as a live `translateY` on the dragged pill so it follows the pointer smoothly,
 *  while `layer` (snapped to whole rows) is only used for the row it commits into on drop.
 *  Keeping the two separate means the dragged pill never needs to jump between separate
 *  per-row DOM containers mid-drag (which caused a flicker/remount each time it crossed a
 *  row boundary) - it stays in its original row's DOM the whole time and only reflows into
 *  its new row once the drag ends and `regions` re-renders normally. */
export function useRegionDrag(
  regions: Region[], dur: number, trackRef: React.RefObject<HTMLDivElement | null>, rowHeightPx: number,
  onCommit: (id: string, start: number, end: number, layer: number) => void, onSel: (id: string) => void,
) {
  const startX = useRef(0);
  const startY = useRef(0);
  const origLayer = useRef(0);
  const [drag, setDrag] = useState<Drag | null>(null);
  // Mutable per-move inputs, updated every render but read from refs inside the effect below -
  // so the effect itself only needs to depend on WHETHER a drag is active, not on `dur`/`onCommit`
  // (a fresh inline arrow from the caller every render)/`rowHeightPx`. Without this, the effect
  // re-ran on every `setDrag` inside `move` - i.e. every pointermove - tearing down and re-adding
  // both window listeners each frame, times however many `useRegionDrag` instances are mounted.
  const durRef = useRef(dur); durRef.current = dur;
  const onCommitRef = useRef(onCommit); onCommitRef.current = onCommit;
  const rowHeightRef = useRef(rowHeightPx); rowHeightRef.current = rowHeightPx;

  const beginDrag = useCallback((e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => {
    e.stopPropagation();
    startX.current = e.clientX;
    startY.current = e.clientY;
    onSel(id);
    const lay = regions.find((z) => z.id === id)?.layer ?? 0;
    origLayer.current = lay;
    const others = regions.filter((z) => z.id !== id && (z.layer ?? 0) === lay);
    const lo = others.filter((z) => z.end_ms <= s).reduce((m, z) => Math.max(m, z.end_ms), 0);
    const hi = others.filter((z) => z.start_ms >= en).reduce((m, z) => Math.min(m, z.start_ms), dur);
    setDrag({ id, mode, oStart: s, oEnd: en, start: s, end: en, lo, hi, layer: lay, dyPx: 0 });
  }, [regions, dur, onSel]);

  // Keyed on drag PRESENCE only (a boolean, not the `Drag` object `setDrag` replaces every move) -
  // attaches the two window listeners once when a drag starts and tears them down once when it
  // ends, instead of once per pointermove.
  const active = drag !== null;
  useEffect(() => {
    if (!active) return;
    const move = (e: PointerEvent) => {
      const el = trackRef.current; if (!el) return;
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * durRef.current;
      const dyPx = e.clientY - startY.current;
      // Dragging UP raises priority (higher layer number), matching the timeline's
      // highest-layer-on-top rendering order (Timeline.tsx).
      const layerDelta = -Math.round(dyPx / rowHeightRef.current);
      setDrag((d) => {
        if (!d) return d;
        const layer = Math.max(0, origLayer.current + layerDelta);
        if (d.mode === "move") {
          const w = d.oEnd - d.oStart; const start = Math.max(d.lo, Math.min(d.hi - w, d.oStart + dms));
          return { ...d, start, end: start + w, layer, dyPx };
        }
        // Resize handles only retime; vertical movement doesn't change layer for them.
        if (d.mode === "l") return { ...d, start: Math.max(d.lo, Math.min(d.oEnd - 150, d.oStart + dms)) };
        return { ...d, end: Math.min(d.hi, Math.max(d.oStart + 150, d.oEnd + dms)) };
      });
    };
    // A bare click (no real movement) must select only, not commit a no-op edit - an undo step +
    // an `update_zoom`/`update_effect`/`update_layout_seg` IPC round trip for nothing (D-Medium
    // M8; UX audit #5). `onSel` above already ran unconditionally on pointerdown, so selection
    // still happens either way.
    const up = (e: PointerEvent) => setDrag((d) => {
      if (d && pastDragThreshold(e.clientX - startX.current, e.clientY - startY.current)) {
        onCommitRef.current(d.id, Math.round(d.start), Math.round(d.end), d.layer);
      }
      return null;
    });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, [active, trackRef]);

  return { drag, beginDrag };
}
