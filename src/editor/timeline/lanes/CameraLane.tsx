import { memo, useCallback, useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import type { EditDoc, EditOp } from "../../../shared/edit";
import type { Rect } from "../../controls/surfaces/popoverPlace";
import { CameraCurvePop } from "./CameraCurvePop";
import { KF_BLEND_MS } from "../../stage/camera/cameraMoves";
import { snapKeyframeMs } from "../model/camSnap";
import { pastDragThreshold } from "../../util/dragThreshold";

const CAM_12 = "color-mix(in srgb, var(--e-cam) 12%, transparent)";

export const CameraLane = memo(function CameraLane({
  doc,
  dur,
  sel,
  onSel,
  onApply,
  track,
  hasWebcam,
}: {
  doc: EditDoc;
  dur: number;
  sel: string | null;
  onSel: (id: string) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  track: React.RefObject<HTMLDivElement | null>;
  hasWebcam: boolean;
}) {
  const startX = useRef(0);
  const startT = useRef(0);
  const movedRef = useRef(false);
  const [drag, setDrag] = useState<{ id: string; t_ms: number } | null>(null);
  const [pick, setPick] = useState<{ id: string; anchor: Rect; easing: string } | null>(null);

  const beginDrag = useCallback(
    (e: React.PointerEvent, id: string, t_ms: number) => {
      e.stopPropagation();
      onSel(id);
      startX.current = e.clientX;
      startT.current = t_ms;
      movedRef.current = false;
      setDrag({ id, t_ms });
    },
    [onSel],
  );

  const durRef = useRef(dur);
  durRef.current = dur;
  const onApplyRef = useRef(onApply);
  onApplyRef.current = onApply;
  const snapRef = useRef<{ segEdges: number[]; otherKfs: number[] }>({ segEdges: [], otherKfs: [] });
  if (drag) {
    snapRef.current = {
      segEdges: doc.layout.flatMap((s) => [s.start_ms, s.end_ms]),
      otherKfs: doc.camera_moves.filter((m) => m.id !== drag.id).map((m) => m.t_ms),
    };
  }

  const active = drag !== null;
  useEffect(() => {
    if (!active) return;
    const move = (e: PointerEvent) => {
      const el = track.current;
      if (!el) return;
      if (!movedRef.current) {
        if (!pastDragThreshold(e.clientX - startX.current, 0)) return;
        movedRef.current = true;
      }
      const dms = ((e.clientX - startX.current) / el.getBoundingClientRect().width) * durRef.current;
      const raw = Math.max(0, Math.min(durRef.current, Math.round(startT.current + dms)));
      const { segEdges, otherKfs } = snapRef.current;
      setDrag((d) => (d ? { id: d.id, t_ms: snapKeyframeMs(raw, segEdges, otherKfs) } : d));
    };
    const up = (e: PointerEvent) =>
      setDrag((d) => {
        if (d && pastDragThreshold(e.clientX - startX.current, 0)) {
          void onApplyRef.current({ op: "update_camera_move", id: d.id, t_ms: d.t_ms });
        }
        return null;
      });
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [active, track]);

  useEffect(() => {
    if (!pick) return;
    const close = (e: Event) => {
      if (!(e.target as HTMLElement)?.closest?.(".e-campop, .e-camseg")) setPick(null);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPick(null);
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", key);
    };
  }, [pick]);

  const pct = (t: number) => (dur > 0 ? (t / dur) * 100 : 0);
  const kfs = doc.camera_moves
    .map((m) => ({ id: m.id, easing: m.easing, t: drag?.id === m.id ? drag.t_ms : m.t_ms }))
    .sort((a, b) => a.t - b.t);
  const closePick = useCallback(() => setPick(null), []);
  const setEasing = (id: string, easing: string) => {
    void onApply({ op: "update_camera_move", id, easing });
    setPick((p) => (p ? { ...p, easing } : p));
  };

  const span =
    kfs.length > 0
      ? { from: Math.max(0, kfs[0].t - KF_BLEND_MS), to: kfs[kfs.length - 1].t + KF_BLEND_MS }
      : null;
  const ramp = span ? (KF_BLEND_MS / Math.max(span.to - span.from, 1)) * 100 : 0;

  return (
    <div className="e-camlane">
      <div className="e-camrow">
        <div className="e-cambase" />
        {hasWebcam && doc.camera_moves.length === 0 && (
          <span className="e-camempty">Turn on Move in preview to keyframe the webcam</span>
        )}
        {span && (
          <div
            className="e-camspan"
            style={{
              left: `${pct(span.from)}%`,
              width: `${pct(span.to - span.from)}%`,
              background:
                `linear-gradient(90deg, transparent 0%, ${CAM_12} ${Math.min(ramp, 50)}%,` +
                ` ${CAM_12} ${Math.max(100 - ramp, 50)}%, transparent 100%)`,
            }}
          />
        )}
        {kfs.slice(1).map((b, i) => {
          const a = kfs[i];
          return (
            <div
              key={`seg-${a.id}-${b.id}`}
              className={`e-camseg${pick?.id === b.id ? " on" : ""}`}
              style={{ left: `${pct(a.t)}%`, width: `${pct(b.t - a.t)}%` }}
              title={`${b.easing.replace(/_/g, " ")} transition - click to change it`}
              onPointerDown={(e) => {
                e.stopPropagation();
                const r = e.currentTarget.getBoundingClientRect();
                setPick({
                  id: b.id,
                  anchor: { left: r.left, top: r.top, width: r.width, height: r.height },
                  easing: b.easing,
                });
              }}
            />
          );
        })}
        {kfs.map((m) => (
          <motion.div
            key={m.id}
            className={`e-camkf${sel === m.id ? " sel" : ""}`}
            style={{ left: `${pct(m.t)}%` }}
            initial={{ opacity: 0, scale: 0.6, rotate: 45 }}
            animate={{ opacity: 1, scale: 1, rotate: 45 }}
            transition={{ type: "tween", duration: 0.14, ease: [0.4, 0, 0.2, 1] }}
            onPointerDown={(e) => beginDrag(e, m.id, m.t)}
          />
        ))}
      </div>
      {pick && (
        <CameraCurvePop
          anchor={pick.anchor}
          easing={pick.easing}
          onPick={(key) => setEasing(pick.id, key)}
          onDismiss={closePick}
        />
      )}
    </div>
  );
});
