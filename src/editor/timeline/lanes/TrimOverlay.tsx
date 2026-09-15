import { useEffect, useRef, useState } from "react";
import type { EditDoc, EditOp, Trim } from "../../../shared/edit";
import { resolveTrim } from "../../../shared/edit";
import { pastDragThreshold } from "../../util/dragThreshold";

type Mode = "in" | "out";
const MIN_GAP_MS = 200;

export function TrimOverlay({
  trim,
  dur,
  trackRef,
  onApply,
}: {
  trim: Trim;
  dur: number;
  trackRef: React.RefObject<HTMLDivElement | null>;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
}) {
  const { inMs, outMs } = resolveTrim(trim, dur);
  const [drag, setDrag] = useState<{ mode: Mode; ms: number } | null>(null);
  const startXRef = useRef(0);
  const movedRef = useRef(false);

  useEffect(() => {
    if (!drag) return;
    const move = (e: PointerEvent) => {
      const el = trackRef.current;
      if (!el || dur <= 0) return;
      if (!movedRef.current) {
        if (!pastDragThreshold(e.clientX - startXRef.current, 0)) return;
        movedRef.current = true;
      }
      const r = el.getBoundingClientRect();
      const raw = Math.round(((e.clientX - r.left) / r.width) * dur);
      setDrag(
        (d) =>
          d && {
            ...d,
            ms:
              d.mode === "in"
                ? Math.max(0, Math.min(raw, outMs - MIN_GAP_MS))
                : Math.max(inMs + MIN_GAP_MS, Math.min(raw, dur)),
          },
      );
    };
    const up = (e: PointerEvent) => {
      setDrag((d) => {
        if (d && pastDragThreshold(e.clientX - startXRef.current, 0)) {
          const nextOut = d.mode === "out" ? d.ms : trim.out_ms;
          void onApply({
            op: "set_trim",
            in_ms: d.mode === "in" ? d.ms : trim.in_ms,
            out_ms: nextOut >= dur ? 0 : nextOut,
          });
        }
        return null;
      });
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [drag, dur, inMs, outMs, trim, onApply, trackRef]);

  if (dur <= 0) return null;
  const liveIn = drag?.mode === "in" ? drag.ms : inMs;
  const liveOut = drag?.mode === "out" ? drag.ms : outMs;
  const pct = (ms: number) => (ms / dur) * 100;

  return (
    <>
      {liveIn > 0 && <div className="e-trimdim" style={{ left: 0, width: `${pct(liveIn)}%` }} />}
      {liveOut < dur && <div className="e-trimdim" style={{ left: `${pct(liveOut)}%`, right: 0 }} />}
      <div
        className={`e-trimhandle${drag?.mode === "in" ? " active" : ""}`}
        style={{ left: `${pct(liveIn)}%` }}
        title="Drag to trim the start"
        onPointerDown={(e) => {
          e.stopPropagation();
          startXRef.current = e.clientX;
          movedRef.current = false;
          setDrag({ mode: "in", ms: liveIn });
        }}
      />
      <div
        className={`e-trimhandle${drag?.mode === "out" ? " active" : ""}`}
        style={{ left: `${pct(liveOut)}%` }}
        title="Drag to trim the end"
        onPointerDown={(e) => {
          e.stopPropagation();
          startXRef.current = e.clientX;
          movedRef.current = false;
          setDrag({ mode: "out", ms: liveOut });
        }}
      />
    </>
  );
}
