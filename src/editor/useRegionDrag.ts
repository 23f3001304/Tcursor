import { useEffect, useRef, useState } from "react";

export type Mode = "move" | "l" | "r";
export interface Drag { id: string; mode: Mode; oStart: number; oEnd: number; start: number; end: number; lo: number; hi: number }
interface Region { id: string; start_ms: number; end_ms: number; layer?: number }

/** Shared drag/resize logic for timeline region pills (zoom now, effects in phase B). Body =
 *  move, side handles = retime; a drag updates a local draft (`drag`) and calls `onCommit` on
 *  release. The draft is clamped to the gap between its same-track neighbours so it can't
 *  overlap them - so overlapping effects must live on different layers (different `regions`). */
export function useRegionDrag(
  regions: Region[], dur: number, trackRef: React.RefObject<HTMLDivElement | null>,
  onCommit: (id: string, start: number, end: number) => void, onSel: (id: string) => void,
) {
  const startX = useRef(0);
  const [drag, setDrag] = useState<Drag | null>(null);

  const beginDrag = (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => {
    e.stopPropagation();
    startX.current = e.clientX;
    onSel(id);
    // Clamp against same-layer neighbours only: regions on other layers may overlap in time.
    const lay = regions.find((z) => z.id === id)?.layer ?? 0;
    const others = regions.filter((z) => z.id !== id && (z.layer ?? 0) === lay);
    const lo = others.filter((z) => z.end_ms <= s).reduce((m, z) => Math.max(m, z.end_ms), 0);
    const hi = others.filter((z) => z.start_ms >= en).reduce((m, z) => Math.min(m, z.start_ms), dur);
    setDrag({ id, mode, oStart: s, oEnd: en, start: s, end: en, lo, hi });
  };

  useEffect(() => {
    if (!drag) return;
    const move = (e: PointerEvent) => {
      const el = trackRef.current; if (!el) return;
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * dur;
      setDrag((d) => {
        if (!d) return d;
        if (d.mode === "move") { const w = d.oEnd - d.oStart; const start = Math.max(d.lo, Math.min(d.hi - w, d.oStart + dms)); return { ...d, start, end: start + w }; }
        if (d.mode === "l") return { ...d, start: Math.max(d.lo, Math.min(d.oEnd - 150, d.oStart + dms)) };
        return { ...d, end: Math.min(d.hi, Math.max(d.oStart + 150, d.oEnd + dms)) };
      });
    };
    const up = () => setDrag((d) => { if (d) onCommit(d.id, Math.round(d.start), Math.round(d.end)); return null; });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, [drag, dur, onCommit, trackRef]);

  return { drag, beginDrag };
}
