import { useEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { pastDragThreshold } from "../hooks/dragThreshold";
import { useRafCoalesced } from "../hooks/useRafCoalesced";

/** An inclusive stretch of CLIP ms, ordered low to high: what the transport's Cut and Speed act on
 *  when one is selected. Owned by `Editor.tsx` so both the timeline (draws it) and the transport
 *  (acts on it) see the same value. */
export type Range = [number, number];

/** The subset of a `React.PointerEvent` this gesture reads - kept structural so the hook can be
 *  driven from a plain object in a test, the way `keymap.ts`' `KeyLike` is. */
export interface RangePointer { clientX: number; shiftKey: boolean; preventDefault(): void; stopPropagation(): void }

/** The ordered clip-ms pair two x positions describe against `rect`, clamped into `[0, dur]`. */
export function rangeOf(ax: number, bx: number, rect: { left: number; width: number }, dur: number): Range {
  const at = (x: number) => Math.round(Math.min(dur, Math.max(0, ((x - rect.left) / rect.width) * dur)));
  const [a, b] = [at(ax), at(bx)];
  return a <= b ? [a, b] : [b, a];
}

/** Shift+drag on the ruler selects a range; a plain drag is declined so it falls through to the
 *  scrub. The returned handler answers whether it claimed the pointerdown.
 *
 *  The live range is written straight into `setRange` (rAF-coalesced, so a high-poll-rate
 *  pointermove burst costs at most one render per frame) rather than held as a second local draft:
 *  one value, so the overlay can never disagree with what the transport would act on. The drag
 *  threshold gates the first write, so a bare Shift+click clears the range instead of leaving a
 *  zero-width one behind, and Escape clears it from anywhere. */
export function useRangeSelect({ dur, trackRef, range, setRange }: {
  dur: number;
  trackRef: RefObject<HTMLDivElement | null>;
  range: Range | null;
  setRange: (r: Range | null) => void;
}): (e: RangePointer) => boolean {
  const anchorX = useRef<number | null>(null);
  const [dragging, setDragging] = useState(false);
  // Mutable per-move inputs read from refs inside the coalesced callback and the window listeners,
  // so neither depends on a value that changes every render (`useRegionDrag`'s own convention).
  const durRef = useRef(dur); durRef.current = dur;
  const setRangeRef = useRef(setRange); setRangeRef.current = setRange;

  const { schedule, flush } = useRafCoalesced((clientX: number) => {
    const el = trackRef.current, a = anchorX.current;
    if (!el || a === null || durRef.current <= 0) return;
    if (!pastDragThreshold(clientX - a, 0)) return; // a bare click leaves the cleared range cleared
    setRangeRef.current(rangeOf(a, clientX, el.getBoundingClientRect(), durRef.current));
  });

  // Keyed on drag PRESENCE, so the two window listeners attach once per drag rather than once per
  // pointermove (the fix `useRegionDrag.md` documents).
  useEffect(() => {
    if (!dragging) return;
    const move = (e: PointerEvent) => schedule(e.clientX);
    const up = (e: PointerEvent) => { schedule(e.clientX); flush(); anchorX.current = null; setDragging(false); };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, [dragging, schedule, flush]);

  // Escape drops the range. Only bound while there is one, so a surface that owns Escape for its
  // own dismiss keeps it whenever nothing is selected here.
  useEffect(() => {
    if (!range) return;
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") setRangeRef.current(null); };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [range]);

  return (e: RangePointer): boolean => {
    if (!e.shiftKey || dur <= 0) return false;
    e.preventDefault();   // without it the shift-drag runs the browser's own text selection
    e.stopPropagation();
    anchorX.current = e.clientX;
    setRangeRef.current(null); // the previous range goes the moment a new gesture starts
    setDragging(true);
    return true;
  };
}
