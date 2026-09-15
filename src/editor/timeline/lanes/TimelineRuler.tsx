import { useRef } from "react";
import type { RefObject } from "react";
import { useRafCoalesced } from "../../hooks/stage/useRafCoalesced";
import { rulerTicks } from "../model/time";
import { useRangeSelect, type Range } from "../useRangeSelect";

export function useSeek(
  dur: number,
  trackRef: RefObject<HTMLDivElement | null>,
  onSeek: (ms: number) => void,
) {
  const seekAt = (clientX: number) => {
    const el = trackRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    onSeek(Math.min(dur, Math.max(0, ((clientX - r.left) / r.width) * dur)));
  };
  const { schedule, flush } = useRafCoalesced(seekAt);
  return { seekAt, scheduleSeek: schedule, flushSeek: flush };
}

export function Ruler({
  dur,
  trackRef,
  onSeek,
  range,
  setRange,
}: {
  dur: number;
  trackRef: RefObject<HTMLDivElement | null>;
  onSeek: (ms: number) => void;
  range: Range | null;
  setRange: (r: Range | null) => void;
}) {
  const { seekAt, scheduleSeek, flushSeek } = useSeek(dur, trackRef, onSeek);
  const beginRange = useRangeSelect({ dur, trackRef, range, setRange });
  const scrubbing = useRef(false);
  return (
    <div
      className="e-ruler"
      title="Drag to scrub. Shift+drag to choose a range."
      onPointerDown={(e) => {
        if (beginRange(e)) return;
        scrubbing.current = true;
        e.currentTarget.setPointerCapture(e.pointerId);
        seekAt(e.clientX);
      }}
      onPointerMove={(e) => {
        if (scrubbing.current && e.buttons === 1) scheduleSeek(e.clientX);
      }}
      onPointerUp={() => {
        scrubbing.current = false;
        flushSeek();
      }}
      onLostPointerCapture={() => {
        scrubbing.current = false;
        flushSeek();
      }}
    >
      {rulerTicks(dur).map((t, i) => (
        <span key={i} style={{ left: `${dur > 0 ? (t.at / dur) * 100 : 0}%` }}>
          {t.label}
        </span>
      ))}
    </div>
  );
}

export function RangeOverlay({ range, dur }: { range: Range | null; dur: number }) {
  if (!range || dur <= 0 || range[1] <= range[0]) return null;
  const left = (Math.max(0, range[0]) / dur) * 100;
  const width = ((Math.min(dur, range[1]) - Math.max(0, range[0])) / dur) * 100;
  return <div className="e-range" style={{ left: `${left}%`, width: `${width}%` }} />;
}
