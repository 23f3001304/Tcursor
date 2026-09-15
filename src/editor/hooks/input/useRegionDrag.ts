import { useCallback, useEffect, useRef, useState } from "react";
import { pastDragThreshold } from "../../util/dragThreshold";

export type Mode = "move" | "l" | "r";
export interface Drag {
  id: string;
  mode: Mode;
  oStart: number;
  oEnd: number;
  start: number;
  end: number;
  lo: number;
  hi: number;
  layer: number;
  dyPx: number;
}

export type BeginDrag = (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => void;
interface Region {
  id: string;
  start_ms: number;
  end_ms: number;
  layer?: number;
}

export function useRegionDrag(
  regions: Region[],
  dur: number,
  trackRef: React.RefObject<HTMLDivElement | null>,
  rowHeightPx: number,
  onCommit: (id: string, start: number, end: number, layer: number) => void,
  onSel: (id: string) => void,
) {
  const startX = useRef(0);
  const startY = useRef(0);
  const origLayer = useRef(0);
  const [drag, setDrag] = useState<Drag | null>(null);
  const durRef = useRef(dur);
  durRef.current = dur;
  const onCommitRef = useRef(onCommit);
  onCommitRef.current = onCommit;
  const rowHeightRef = useRef(rowHeightPx);
  rowHeightRef.current = rowHeightPx;

  const beginDrag = useCallback(
    (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => {
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
    },
    [regions, dur, onSel],
  );

  const active = drag !== null;
  useEffect(() => {
    if (!active) return;
    const move = (e: PointerEvent) => {
      const el = trackRef.current;
      if (!el) return;
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * durRef.current;
      const dyPx = e.clientY - startY.current;
      const layerDelta = -Math.round(dyPx / rowHeightRef.current);
      setDrag((d) => {
        if (!d) return d;
        const layer = Math.max(0, origLayer.current + layerDelta);
        if (d.mode === "move") {
          const w = d.oEnd - d.oStart;
          const start = Math.max(d.lo, Math.min(d.hi - w, d.oStart + dms));
          return { ...d, start, end: start + w, layer, dyPx };
        }
        if (d.mode === "l") return { ...d, start: Math.max(d.lo, Math.min(d.oEnd - 150, d.oStart + dms)) };
        return { ...d, end: Math.min(d.hi, Math.max(d.oStart + 150, d.oEnd + dms)) };
      });
    };
    const up = (e: PointerEvent) =>
      setDrag((d) => {
        if (d && pastDragThreshold(e.clientX - startX.current, e.clientY - startY.current)) {
          onCommitRef.current(d.id, Math.round(d.start), Math.round(d.end), d.layer);
        }
        return null;
      });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [active, trackRef]);

  return { drag, beginDrag };
}
