import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { CAM_CURVES } from "../inspectors/curves";
import { KF_BLEND_MS } from "../stage/cameraMoves";
import { snapKeyframeMs } from "./camSnap";

/** Camera-move keyframe lane: a diamond per doc.camera_moves entry at (t_ms/dur)*100%, over a
 *  dashed baseline (the track). A glowing segment is drawn between each consecutive pair - that's
 *  where the PiP animates; clicking it opens a compact curve popover to set that transition's
 *  easing (into the later keyframe), the same choice the keyframe inspector offers. Behind it all,
 *  a translucent span bar marks what the keyframes actually OWN - `[first - KF_BLEND_MS,
 *  last + KF_BLEND_MS]` - with the two blend windows drawn as the fade ramps of a static CSS
 *  gradient (no Motion: nothing here moves). Outside that bar the layout segments own the webcam.
 *  Single-point keyframes, not regions (no useRegionDrag): click a diamond selects, horizontal
 *  drag retimes (snapped to layout-segment edges and sibling keyframes, committed on release).
 *  Motion owns the diamond transform (scale + rotate), so it centers by margin. */
export function CameraLane({ doc, dur, sel, onSel, onApply, track }: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}) {
  const startX = useRef(0);
  const startT = useRef(0);
  const [drag, setDrag] = useState<{ id: string; t_ms: number } | null>(null);
  // Open transition-curve popover: the destination keyframe id, the anchor % (segment midpoint),
  // and its current easing (for the live highlight). null = closed.
  const [pick, setPick] = useState<{ id: string; leftPct: number; easing: string } | null>(null);

  const beginDrag = (e: React.PointerEvent, id: string, t_ms: number) => {
    e.stopPropagation();
    onSel(id);
    startX.current = e.clientX;
    startT.current = t_ms;
    setDrag({ id, t_ms });
  };

  useEffect(() => {
    if (!drag) return;
    // Snap targets, in ms space so the feel is zoom-independent: every layout-segment edge, plus
    // every OTHER keyframe's time (its own would pin it where the drag started).
    const segEdges = doc.layout.flatMap((s) => [s.start_ms, s.end_ms]);
    const otherKfs = doc.camera_moves.filter((m) => m.id !== drag.id).map((m) => m.t_ms);
    const move = (e: PointerEvent) => {
      const el = track.current; if (!el) return;
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * dur;
      const raw = Math.max(0, Math.min(dur, Math.round(startT.current + dms)));
      setDrag((d) => d ? { id: d.id, t_ms: snapKeyframeMs(raw, segEdges, otherKfs) } : d);
    };
    const up = () => setDrag((d) => { if (d) void onApply({ op: "update_camera_move", id: d.id, t_ms: d.t_ms }); return null; });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, [doc, drag, dur, onApply, track]);

  // Dismiss the curve popover on any outside pointerdown / Escape.
  useEffect(() => {
    if (!pick) return;
    // Ignore clicks on a segment too, else clicking another segment closes here right after its
    // own handler reopened the popover for that segment.
    const close = (e: Event) => { if (!(e.target as HTMLElement)?.closest?.(".e-campop, .e-camseg")) setPick(null); };
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") setPick(null); };
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", key);
    return () => { window.removeEventListener("pointerdown", close); window.removeEventListener("keydown", key); };
  }, [pick]);

  // Effective times (live drag draft applied), sorted - diamonds AND the segments between them
  // follow the dragged keyframe and re-order when it crosses a neighbour.
  const pct = (t: number) => (dur > 0 ? (t / dur) * 100 : 0);
  const kfs = doc.camera_moves
    .map((m) => ({ id: m.id, easing: m.easing, t: drag?.id === m.id ? drag.t_ms : m.t_ms }))
    .sort((a, b) => a.t - b.t);
  // Commit a curve to the destination keyframe; keep the popover open so curves can be auditioned.
  const setEasing = (id: string, easing: string) => {
    void onApply({ op: "update_camera_move", id, easing });
    setPick((p) => p ? { ...p, easing } : p);
  };

  // The span the track owns, drag-draft included so the bar follows the diamond being moved.
  // Its two BLEND-long ends are the gradient's fade ramps; the middle sits at a flat 12% alpha.
  const span = kfs.length > 0
    ? { from: Math.max(0, kfs[0].t - KF_BLEND_MS), to: kfs[kfs.length - 1].t + KF_BLEND_MS } : null;
  const ramp = span ? (KF_BLEND_MS / Math.max(span.to - span.from, 1)) * 100 : 0;

  return (
    <div className="e-camlane">
      <div className="e-camrow">
        <div className="e-cambase" />
        {span && (
          <div className="e-camspan" style={{ left: `${pct(span.from)}%`, width: `${pct(span.to - span.from)}%`,
            background: `linear-gradient(90deg, rgba(224,93,158,0) 0%, rgba(224,93,158,.12) ${Math.min(ramp, 50)}%,`
              + ` rgba(224,93,158,.12) ${Math.max(100 - ramp, 50)}%, rgba(224,93,158,0) 100%)` }} />
        )}
        {kfs.slice(1).map((b, i) => {
          const a = kfs[i];
          return (
            <div key={`seg-${a.id}-${b.id}`} className={`e-camseg${pick?.id === b.id ? " on" : ""}`}
              style={{ left: `${pct(a.t)}%`, width: `${pct(b.t - a.t)}%` }}
              title={`${b.easing.replace(/_/g, " ")} transition - click to change it`}
              onPointerDown={(e) => { e.stopPropagation(); setPick({ id: b.id, leftPct: (pct(a.t) + pct(b.t)) / 2, easing: b.easing }); }} />
          );
        })}
        {kfs.map((m) => (
          <motion.div key={m.id} className={`e-camkf${sel === m.id ? " sel" : ""}`}
            style={{ left: `${pct(m.t)}%` }}
            initial={{ opacity: 0, scale: 0.6, rotate: 45 }}
            animate={{ opacity: 1, scale: 1, rotate: 45 }}
            whileHover={{ scale: 1.2, rotate: 45 }}
            transition={{ type: "tween", duration: 0.14, ease: [0.4, 0, 0.2, 1] }}
            onPointerDown={(e) => beginDrag(e, m.id, m.t)} />
        ))}
      </div>
      {pick && (
        <div className="e-campop" style={{ left: `${pick.leftPct}%` }}>
          {CAM_CURVES.map((c) => (
            <button key={c.key} type="button" title={c.name}
              className={`e-campop-b${pick.easing === c.key ? " on" : ""}`}
              onClick={() => setEasing(pick.id, c.key)}>
              <svg viewBox="0 -30 100 160"><path d={c.path} fill="none"
                stroke={pick.easing === c.key ? "var(--e-fg)" : "var(--e-mut)"} strokeWidth="8" strokeLinecap="round" /></svg>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
