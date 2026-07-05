import { useRef } from "react";
import { motion } from "motion/react";
import { IconZoomIn, IconBulb, IconAspectRatio } from "@tabler/icons-react";
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
const prettyLayout = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

export function Timeline({ doc, timeMs, dur, onSeek, sel, onSel, onApply, thumbs, waves }: {
  doc: EditDoc; timeMs: number; dur: number; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string };
}) {
  const track = useRef<HTMLDivElement>(null);
  // Zooms, laid out into layers so overlapping ones stack on separate rows instead of colliding.
  const zooms = layoutRegions(doc.zooms);
  const { drag, beginDrag } = useRegionDrag(zooms, dur, track, 34,
    (id, start_ms, end_ms, layer) => void onApply({ op: "update_zoom", id, start_ms, end_ms, layer }), onSel);
  const maxZoomLayer = Math.max(
    ...zooms.map((z) => z.layer),
    drag ? drag.layer : 0
  );
  const zoomRows = zooms.length ? maxZoomLayer + 1 : 0;

  // Effect regions, laid out into layers so overlapping ones stack on separate rows.
  const fx = layoutRegions(doc.effects);
  const eff = useRegionDrag(fx, dur, track, 30,
    (id, start_ms, end_ms, layer) => void onApply({ op: "update_effect", id, start_ms, end_ms, layer }), onSel);
  const maxFxLayer = Math.max(
    ...fx.map((f) => f.layer),
    eff.drag ? eff.drag.layer : 0
  );
  const fxRows = fx.length ? maxFxLayer + 1 : 0;

  // Layout regions, laid out into layers so overlapping ones stack on separate rows. Layout
  // segments have no priority layer (update_layout_seg takes no `layer`), so onCommit only
  // forwards id/start/end - the layer arg useRegionDrag reports is used for row-stacking only.
  // "screen" IS the empty default (gaps render as screen), so a screen segment is meaningless as a
  // pill - hide it from the track; only non-screen layouts show as editable pills.
  const layouts = layoutRegions(doc.layout.filter((s) => s.layout !== "screen"));
  const lay = useRegionDrag(layouts, dur, track, 30,
    (id, start_ms, end_ms) => void onApply({ op: "update_layout_seg", id, start_ms, end_ms }), onSel);
  const maxLayLayer = Math.max(
    ...layouts.map((l) => l.layer),
    lay.drag ? lay.drag.layer : 0
  );
  const layRows = layouts.length ? maxLayLayer + 1 : 0;

  const seekAt = (clientX: number) => {
    const el = track.current; if (!el) return;
    const r = el.getBoundingClientRect();
    onSeek(Math.min(dur, Math.max(0, ((clientX - r.left) / r.width) * dur)));
  };
  const pct = dur > 0 ? (timeMs / dur) * 100 : 0;
  const span = (z: { id: string; start_ms: number; end_ms: number }) =>
    drag && drag.id === z.id ? { s: drag.start, e: drag.end } : { s: z.start_ms, e: z.end_ms };

  // The dragged pill stays in its ORIGINAL row's DOM the whole drag (grouped by the static
  // z.layer/f.layer, not the live snapped target) and instead visually follows the pointer via
  // its own `y` motion value at zero-duration (instant, no spring lag) - moving it into a
  // different row's <div> live would remount it (a different React parent), replaying its
  // mount fade-in every time it crossed a row boundary. It only actually reflows into the new
  // row once the drag ends and the layer commits for real.

  return (
    <div className="e-timeline">
      <div className="e-ruler">
        {rulerTicks(dur).map((t, i) => (
          <span key={i} style={{ left: `${dur > 0 ? (t.at / dur) * 100 : 0}%` }}>{t.label}</span>
        ))}
      </div>
      <div className="e-tlbody" ref={track}
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
        }}
        onDrop={(e) => {
          e.preventDefault();
          const type = e.dataTransfer.getData("text/plain");
          const el = track.current; if (!el) return;
          const rect = el.getBoundingClientRect();
          const dropMs = Math.round(Math.min(dur, Math.max(0, ((e.clientX - rect.left) / rect.width) * dur)));
          if (type === "zoom") {
            void onApply({ op: "add_zoom", at_ms: dropMs, dur_ms: 2000 });
          } else if (type === "spotlight") {
            void onApply({ op: "add_effect", kind: "spotlight", start_ms: dropMs, end_ms: dropMs + 2000 });
          } else if (type === "layout") {
            void onApply({ op: "add_layout_seg", at_ms: dropMs, dur_ms: 2000, layout: "camera" });
          }
        }}
        onPointerDown={(e) => { if (!drag) { e.currentTarget.setPointerCapture(e.pointerId); seekAt(e.clientX); } }}
        onPointerMove={(e) => { if (!drag && e.buttons === 1) seekAt(e.clientX); }}>
        <Filmstrip thumbs={thumbs} />
        <div className="e-tracks">
          {Array.from({ length: zoomRows }, (_, i) => zoomRows - 1 - i).map((layer) => (
            <div className="e-zoomrow" key={`zoom${layer}`}>
              {zooms.filter((z) => z.layer === layer).map((z) => {
                const { s, e } = span(z);
                const dragging = drag?.id === z.id;
                return (
                  <motion.div key={z.id} className={`e-zblk${sel === z.id ? " sel" : ""}${dragging ? " drag" : ""}`}
                    style={{ left: `${(s / dur) * 100}%`, width: `${Math.max(2.5, ((e - s) / dur) * 100)}%` }}
                    initial={{ opacity: 0 }} animate={{ opacity: 1, y: dragging ? drag.dyPx : 0 }}
                    transition={{ opacity: { type: "spring", stiffness: 480, damping: 30 },
                      y: dragging ? { duration: 0 } : { type: "spring", stiffness: 480, damping: 30 } }}
                    onPointerDown={(ev) => beginDrag(ev, z.id, "move", z.start_ms, z.end_ms)}>
                    <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, z.id, "l", z.start_ms, z.end_ms)} />
                    <span className="e-zlabel"><IconZoomIn size={12} />{z.scale.toFixed(1)}x</span>
                    <span className="e-zh" onPointerDown={(ev) => beginDrag(ev, z.id, "r", z.start_ms, z.end_ms)} />
                  </motion.div>
                );
              })}
            </div>
          ))}
          {Array.from({ length: fxRows }, (_, i) => fxRows - 1 - i).map((layer) => (
            <div className="e-fxrow" key={`fx${layer}`}>
              {fx.filter((f) => f.layer === layer).map((f) => {
                const dragging = eff.drag?.id === f.id;
                const s = dragging && eff.drag ? eff.drag.start : f.start_ms;
                const e = dragging && eff.drag ? eff.drag.end : f.end_ms;
                return (
                  <motion.div key={f.id} className={`e-fxblk${sel === f.id ? " sel" : ""}${dragging ? " drag" : ""}`}
                    style={{ left: `${(s / dur) * 100}%`, width: `${Math.max(2.5, ((e - s) / dur) * 100)}%` }}
                    initial={{ opacity: 0 }} animate={{ opacity: 1, y: dragging && eff.drag ? eff.drag.dyPx : 0 }}
                    transition={{ opacity: { type: "spring", stiffness: 480, damping: 30 },
                      y: dragging ? { duration: 0 } : { type: "spring", stiffness: 480, damping: 30 } }}
                    onPointerDown={(ev) => eff.beginDrag(ev, f.id, "move", f.start_ms, f.end_ms)}>
                    <span className="e-zh" onPointerDown={(ev) => eff.beginDrag(ev, f.id, "l", f.start_ms, f.end_ms)} />
                    <span className="e-zlabel"><IconBulb size={12} />Spotlight</span>
                    <span className="e-zh" onPointerDown={(ev) => eff.beginDrag(ev, f.id, "r", f.start_ms, f.end_ms)} />
                  </motion.div>
                );
              })}
            </div>
          ))}
          {Array.from({ length: layRows }, (_, i) => layRows - 1 - i).map((layer) => (
            <div className="e-layrow" key={`lay${layer}`}>
              {layouts.filter((l) => l.layer === layer).map((l) => {
                const dragging = lay.drag?.id === l.id;
                const s = dragging && lay.drag ? lay.drag.start : l.start_ms;
                const e = dragging && lay.drag ? lay.drag.end : l.end_ms;
                return (
                  <motion.div key={l.id} className={`e-layblk${sel === l.id ? " sel" : ""}${dragging ? " drag" : ""}`}
                    style={{ left: `${(s / dur) * 100}%`, width: `${Math.max(2.5, ((e - s) / dur) * 100)}%` }}
                    initial={{ opacity: 0 }} animate={{ opacity: 1, y: dragging && lay.drag ? lay.drag.dyPx : 0 }}
                    transition={{ opacity: { type: "spring", stiffness: 480, damping: 30 },
                      y: dragging ? { duration: 0 } : { type: "spring", stiffness: 480, damping: 30 } }}
                    onPointerDown={(ev) => lay.beginDrag(ev, l.id, "move", l.start_ms, l.end_ms)}>
                    <span className="e-zh" onPointerDown={(ev) => lay.beginDrag(ev, l.id, "l", l.start_ms, l.end_ms)} />
                    <span className="e-zlabel"><IconAspectRatio size={12} />{prettyLayout(l.layout)}</span>
                    <span className="e-zh" onPointerDown={(ev) => lay.beginDrag(ev, l.id, "r", l.start_ms, l.end_ms)} />
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
