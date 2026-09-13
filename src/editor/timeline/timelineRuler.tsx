import { useRef } from "react";
import type { RefObject } from "react";
import { useRafCoalesced } from "../hooks/useRafCoalesced";
import { rulerTicks } from "./time";
import { useRangeSelect, type Range } from "./useRangeSelect";

/** The timeline's own top strip and the scrub math behind it, extracted from `Timeline.tsx` (which
 *  sat on its line cap) so the range gesture had somewhere to live. Both the ruler and the track
 *  body measure against the SAME element (`trackRef`, `.e-tlbody`), so a tick, a pill and a
 *  playhead at the same ms always land on the same x. */

/** Map an x to clip ms against the track's own box, and the rAF-coalesced version a pointermove
 *  burst goes through (at most one seek per frame, always the latest x; `flushSeek` applies a
 *  still-pending one on release). */
export function useSeek(dur: number, trackRef: RefObject<HTMLDivElement | null>, onSeek: (ms: number) => void) {
  const seekAt = (clientX: number) => {
    const el = trackRef.current; if (!el) return;
    const r = el.getBoundingClientRect();
    onSeek(Math.min(dur, Math.max(0, ((clientX - r.left) / r.width) * dur)));
  };
  const { schedule, flush } = useRafCoalesced(seekAt);
  return { seekAt, scheduleSeek: schedule, flushSeek: flush };
}

/** The adaptive tick strip, and the two gestures that start on it: a plain drag scrubs (the track
 *  body's gesture, now reachable from the ruler too), Shift+drag selects a range for the
 *  transport's Cut and Speed actions. */
export function Ruler({ dur, trackRef, onSeek, range, setRange }: {
  dur: number; trackRef: RefObject<HTMLDivElement | null>; onSeek: (ms: number) => void;
  range: Range | null; setRange: (r: Range | null) => void;
}) {
  const { seekAt, scheduleSeek, flushSeek } = useSeek(dur, trackRef, onSeek);
  const beginRange = useRangeSelect({ dur, trackRef, range, setRange });
  const scrubbing = useRef(false);
  return (
    <div className="e-ruler" title="Drag to scrub. Shift+drag to choose a range."
      onPointerDown={(e) => {
        if (beginRange(e)) return; // Shift+drag claimed it
        scrubbing.current = true;
        e.currentTarget.setPointerCapture(e.pointerId);
        seekAt(e.clientX);
      }}
      onPointerMove={(e) => { if (scrubbing.current && e.buttons === 1) scheduleSeek(e.clientX); }}
      onPointerUp={() => { scrubbing.current = false; flushSeek(); }}
      onLostPointerCapture={() => { scrubbing.current = false; flushSeek(); }}>
      {rulerTicks(dur).map((t, i) => (
        <span key={i} style={{ left: `${dur > 0 ? (t.at / dur) * 100 : 0}%` }}>{t.label}</span>
      ))}
    </div>
  );
}

/** The selected range, drawn across the whole track body so what Cut and Speed will act on reads
 *  at a glance. A translucent accent wash with a 1px accent edge at each end, no border and no
 *  handles: the gesture that made it is how it is changed. Static CSS, nothing here animates. */
export function RangeOverlay({ range, dur }: { range: Range | null; dur: number }) {
  if (!range || dur <= 0 || range[1] <= range[0]) return null;
  const left = (Math.max(0, range[0]) / dur) * 100;
  const width = ((Math.min(dur, range[1]) - Math.max(0, range[0])) / dur) * 100;
  return <div className="e-range" style={{ left: `${left}%`, width: `${width}%` }} />;
}
