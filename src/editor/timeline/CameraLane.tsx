import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import type { EditDoc, EditOp } from "../../lib/edit";

/** Camera-move keyframe lane: one diamond per `doc.camera_moves` entry, positioned at
 *  `(t_ms/dur)*100%`. Unlike the zoom/effect/layout tracks these are single-point
 *  keyframes (not start/end regions), so there's no `useRegionDrag` region and no
 *  layer-stacking - a single row holds every diamond. Click selects; horizontal drag
 *  retimes (a local draft while dragging, committed as `update_camera_move` on release). */
export function CameraLane({ doc, dur, sel, onSel, onApply, track }: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}) {
  const startX = useRef(0);
  const startT = useRef(0);
  const [drag, setDrag] = useState<{ id: string; t_ms: number } | null>(null);

  const beginDrag = (e: React.PointerEvent, id: string, t_ms: number) => {
    e.stopPropagation();
    onSel(id);
    startX.current = e.clientX;
    startT.current = t_ms;
    setDrag({ id, t_ms });
  };

  useEffect(() => {
    if (!drag) return;
    const move = (e: PointerEvent) => {
      const el = track.current; if (!el) return;
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * dur;
      setDrag((d) => d ? { id: d.id, t_ms: Math.max(0, Math.min(dur, Math.round(startT.current + dms))) } : d);
    };
    const up = () => setDrag((d) => { if (d) void onApply({ op: "update_camera_move", id: d.id, t_ms: d.t_ms }); return null; });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, [drag, dur, onApply, track]);

  return (
    <div className="e-camrow">
      {doc.camera_moves.map((m) => {
        const t = drag?.id === m.id ? drag.t_ms : m.t_ms;
        return (
          <motion.div key={m.id} className={`e-camkf${sel === m.id ? " sel" : ""}`}
            style={{ left: `${dur > 0 ? (t / dur) * 100 : 0}%` }}
            initial={{ opacity: 0, scale: 0.6 }} whileHover={{ scale: 1.15 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ type: "tween", duration: 0.14, ease: [0.4, 0, 0.2, 1] }}
            onPointerDown={(e) => beginDrag(e, m.id, m.t_ms)} />
        );
      })}
    </div>
  );
}
