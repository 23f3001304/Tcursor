import { useCallback, useEffect, useRef, useState } from "react";
import { pastDragThreshold } from "../../util/dragThreshold";
import type { Drag, Mode } from "./useRegionDrag";

export const MIN_CLIP_MS = 100;

interface ClipLike {
  id: string;
  start_ms: number;
  end_ms: number;
  layer: number;
}

export function useClipDrag(
  regions: ClipLike[],
  dur: number,
  trackRef: React.RefObject<HTMLDivElement | null>,
  onReorder: (id: string, atMs: number) => void,
  onRetime: (id: string, srcIn: number, srcOut: number) => void,
  onSel: (id: string) => void,
) {
  const startX = useRef(0);
  const startY = useRef(0);
  const [drag, setDrag] = useState<Drag | null>(null);
  const durRef = useRef(dur);
  durRef.current = dur;
  const cbRef = useRef({ onReorder, onRetime });
  cbRef.current = { onReorder, onRetime };

  const beginDrag = useCallback(
    (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => {
      e.stopPropagation();
      startX.current = e.clientX;
      startY.current = e.clientY;
      onSel(id);
      const lay = regions.find((r) => r.id === id)?.layer ?? 0;
      setDrag({ id, mode, oStart: s, oEnd: en, start: s, end: en, lo: 0, hi: dur, layer: lay, dyPx: 0 });
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
      setDrag((d) => {
        if (!d) return d;
        if (d.mode === "move") {
          const w = d.oEnd - d.oStart;
          const start = Math.max(0, Math.min(durRef.current - w, d.oStart + dms));
          return { ...d, start, end: start + w };
        }
        if (d.mode === "l")
          return { ...d, start: Math.max(0, Math.min(d.oEnd - MIN_CLIP_MS, d.oStart + dms)) };
        return { ...d, end: Math.min(durRef.current, Math.max(d.oStart + MIN_CLIP_MS, d.oEnd + dms)) };
      });
    };
    const up = (e: PointerEvent) =>
      setDrag((d) => {
        if (d && pastDragThreshold(e.clientX - startX.current, e.clientY - startY.current)) {
          const c = cbRef.current;
          if (d.mode === "move") c.onReorder(d.id, Math.round((d.start + d.end) / 2));
          else c.onRetime(d.id, Math.round(d.start), Math.round(d.end));
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
