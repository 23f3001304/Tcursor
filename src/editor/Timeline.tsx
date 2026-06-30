import { useRef } from "react";
import { motion } from "motion/react";
import { IconZoomIn, IconBulb } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../lib/edit";
import { rulerTicks } from "./time";
import { useRegionDrag } from "./useRegionDrag";
import { layoutRegions } from "./layers";
import { Filmstrip } from "./Filmstrip";
import { AudioTrack } from "./AudioTrack";

/** Multi-track timeline (Filmora-style): an adaptive ruler, a filmstrip clip, and a scrolling
 *  stack of tracks - the zoom track (pills drag/resize via useRegionDrag) plus the system + mic
 *  audio waveforms - with a playhead spanning the clip. Selecting a pill opens the inspector;
 *  add-zoom lives in the transport. The playhead lives inside the body so it stays aligned with
 *  the ruler ticks + pills regardless of the timeline's outer padding. */
export function Timeline({ doc, timeMs, dur, onSeek, sel, onSel, onApply, thumbs, waves }: {
  doc: EditDoc; timeMs: number; dur: number; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string };
}) {
  const track = useRef<HTMLDivElement>(null);
  const { drag, beginDrag } = useRegionDrag(doc.zooms, dur, track,
    (id, start_ms, end_ms) => void onApply({ op: "update_zoom", id, start_ms, end_ms }), onSel);
  // Effect regions, laid out into layers so overlapping ones stack on separate rows.
  const fx = layoutRegions(doc.effects);
  const fxRows = fx.length ? Math.max(...fx.map((f) => f.layer)) + 1 : 0;
  const eff = useRegionDrag(fx, dur, track,
    (id, start_ms, end_ms) => void onApply({ op: "update_effect", id, start_ms, end_ms }), onSel);

  const seekAt = (clientX: number) => {
    const el = track.current; if (!el) return;
    const r = el.getBoundingClientRect();
    onSeek(Math.min(dur, Math.max(0, ((clientX - r.left) / r.width) * dur)));
  };
  const pct = dur > 0 ? (timeMs / dur) * 100 : 0;
  const span = (z: { id: string; start_ms: number; end_ms: number }) =>
    drag && drag.id === z.id ? { s: drag.start, e: drag.end } : { s: z.start_ms, e: z.end_ms };

  return (
    <div className="e-timeline">
      <div className="e-ruler">
        {rulerTicks(dur).map((t, i) => (
          <span key={i} style={{ left: `${dur > 0 ? (t.at / dur) * 100 : 0}%` }}>{t.label}</span>
        ))}
      </div>
      <div className="e-tlbody" ref={track}
        onPointerDown={(e) => { if (!drag) { e.currentTarget.setPointerCapture(e.pointerId); seekAt(e.clientX); } }}
        onPointerMove={(e) => { if (!drag && e.buttons === 1) seekAt(e.clientX); }}>
        <Filmstrip thumbs={thumbs} />
        <div className="e-tracks">
          <div className="e-zoomrow">
            {doc.zooms.map((z) => {
              const { s, e } = span(z);
              return (
                <motion.div key={z.id} className={`e-zblk${sel === z.id ? " sel" : ""}${drag?.id === z.id ? " drag" : ""}`}
                  style={{ left: `${(s / dur) * 100}%`, width: `${Math.max(2.5, ((e - s) / dur) * 100)}%` }}
                  initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }}
                  transition={{ type: "spring", stiffness: 480, damping: 30 }}
                  onPointerDown={(ev) => beginDrag(ev, z.id, "move", z.start_ms, z.end_ms)}>
                  <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, z.id, "l", z.start_ms, z.end_ms)} />
                  <span className="e-zlabel"><IconZoomIn size={12} />{z.scale.toFixed(1)}x</span>
                  <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, z.id, "r", z.start_ms, z.end_ms)} />
                </motion.div>
              );
            })}
          </div>
          {Array.from({ length: fxRows }, (_, layer) => (
            <div className="e-fxrow" key={`fx${layer}`}>
              {fx.filter((f) => f.layer === layer).map((f) => {
                const s = eff.drag?.id === f.id ? eff.drag.start : f.start_ms;
                const e = eff.drag?.id === f.id ? eff.drag.end : f.end_ms;
                return (
                  <motion.div key={f.id} className={`e-fxblk${sel === f.id ? " sel" : ""}${eff.drag?.id === f.id ? " drag" : ""}`}
                    style={{ left: `${(s / dur) * 100}%`, width: `${Math.max(2.5, ((e - s) / dur) * 100)}%` }}
                    initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }}
                    transition={{ type: "spring", stiffness: 480, damping: 30 }}
                    onPointerDown={(ev) => eff.beginDrag(ev, f.id, "move", f.start_ms, f.end_ms)}>
                    <span className="e-zh" onPointerDown={(ev) => eff.beginDrag(ev, f.id, "l", f.start_ms, f.end_ms)} />
                    <span className="e-zlabel"><IconBulb size={12} />Spotlight</span>
                    <span className="e-zh" onPointerDown={(ev) => eff.beginDrag(ev, f.id, "r", f.start_ms, f.end_ms)} />
                  </motion.div>
                );
              })}
            </div>
          ))}
          <AudioTrack src={waves.system} kind="system" />
          <AudioTrack src={waves.mic} kind="mic" />
        </div>
        <motion.div className="e-ph" initial={false} animate={{ left: `${pct}%` }}
          transition={{ type: "spring", stiffness: 700, damping: 42 }}><i /></motion.div>
      </div>
    </div>
  );
}
