import { useEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { pastDragThreshold } from "../util/dragThreshold";
import { useRafCoalesced } from "../hooks/stage/useRafCoalesced";

export type Range = [number, number];

export interface RangePointer {
  clientX: number;
  shiftKey: boolean;
  preventDefault(): void;
  stopPropagation(): void;
}

export function rangeOf(ax: number, bx: number, rect: { left: number; width: number }, dur: number): Range {
  const at = (x: number) => Math.round(Math.min(dur, Math.max(0, ((x - rect.left) / rect.width) * dur)));
  const [a, b] = [at(ax), at(bx)];
  return a <= b ? [a, b] : [b, a];
}

export function useRangeSelect({
  dur,
  trackRef,
  range,
  setRange,
}: {
  dur: number;
  trackRef: RefObject<HTMLDivElement | null>;
  range: Range | null;
  setRange: (r: Range | null) => void;
}): (e: RangePointer) => boolean {
  const anchorX = useRef<number | null>(null);
  const [dragging, setDragging] = useState(false);
  const durRef = useRef(dur);
  durRef.current = dur;
  const setRangeRef = useRef(setRange);
  setRangeRef.current = setRange;

  const { schedule, flush } = useRafCoalesced((clientX: number) => {
    const el = trackRef.current,
      a = anchorX.current;
    if (!el || a === null || durRef.current <= 0) return;
    if (!pastDragThreshold(clientX - a, 0)) return;
    setRangeRef.current(rangeOf(a, clientX, el.getBoundingClientRect(), durRef.current));
  });

  useEffect(() => {
    if (!dragging) return;
    const move = (e: PointerEvent) => schedule(e.clientX);
    const up = (e: PointerEvent) => {
      schedule(e.clientX);
      flush();
      anchorX.current = null;
      setDragging(false);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [dragging, schedule, flush]);

  useEffect(() => {
    if (!range) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setRangeRef.current(null);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [range]);

  return (e: RangePointer): boolean => {
    if (!e.shiftKey || dur <= 0) return false;
    e.preventDefault();
    e.stopPropagation();
    anchorX.current = e.clientX;
    setRangeRef.current(null);
    setDragging(true);
    return true;
  };
}
