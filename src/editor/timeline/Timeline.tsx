import { useRef } from "react";
import { motion } from "motion/react";
import { IconZoomIn, IconBulb, IconAspectRatio } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { rulerTicks } from "./time";
import { useRegionDrag } from "../hooks/useRegionDrag";
import { layoutRegions, transitionRampPct } from "./layers";
import { Filmstrip } from "./Filmstrip";
import { AudioTrack } from "./AudioTrack";
import { CameraLane } from "./CameraLane";
import { TrimOverlay } from "./TrimOverlay";
import { RegionRows } from "./RegionRows";

/** Multi-track timeline (Filmora-style): an adaptive ruler, a filmstrip clip, and a scrolling
 *  stack of tracks - the zoom track (pills drag/resize via useRegionDrag) plus the system + mic
 *  audio waveforms - with a playhead spanning the clip. Selecting a pill opens the inspector;
 *  add-zoom lives in the transport. The playhead lives inside the body so it stays aligned with
 *  the ruler ticks + pills regardless of the timeline's outer padding. */
const prettyLayout = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

// Row-height constants shared by the lane label gutter and `.e-tracks`' own natural row layout
// (editor.css uses the same 32/22/6 numbers) - kept in one place so a lane's gutter label slot is
// always exactly as tall as the lane's real content, never computed twice with a chance to drift.
const ROW_H = 32, AUDIO_ROW_H = 22, GAP = 6;
const laneHeight = (rows: number, rowH: number) => (rows > 0 ? rows * rowH + (rows - 1) * GAP : 0);

export function Timeline({ doc, timeMs, dur, playing, onSeek, sel, onSel, onApply, thumbs, waves, wavesReady }: {
  doc: EditDoc; timeMs: number; dur: number; playing: boolean; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean;
}) {
  const track = useRef<HTMLDivElement>(null);
  // Whether a SCRUB (a pointerdown that actually started on the body, not a pill/handle/keyframe
  // bubbling up) is in progress - the only condition `onPointerMove` should chase the pointer
  // under. Every draggable child (zoom/effect/layout pills via `useRegionDrag.beginDrag`, trim
  // handles, camera keyframes) calls `e.stopPropagation()` on its own `pointerdown`, so this ref
  // only ever flips true for a genuine body-originated scrub - it replaces the old `!drag` check,
  // which only ever guarded against the ZOOM lane's own drag and let every OTHER lane's drag
  // bubble through as a `buttons===1` pointermove, chasing the playhead during any non-zoom drag.
  const scrubbing = useRef(false);
  // The label gutter (`.e-lanegutter`) is a SIBLING of `.e-tracks`, not a descendant - `.e-tracks`
  // clips its own descendants (`overflow-y: auto` forces `overflow-x` to clip too, a CSS rule with
  // no per-descendant exception), so a label positioned INSIDE it can never escape that clip no
  // matter how it's offset. As a sibling it isn't clipped at all, but it also doesn't scroll with
  // `.e-tracks` for free - `onTracksScroll` below mirrors `.e-tracks`' scrollTop onto it every
  // scroll event, keeping the two columns visually locked together.
  const gutterRef = useRef<HTMLDivElement>(null);
  const onTracksScroll = (e: React.UIEvent<HTMLDivElement>) => {
    if (gutterRef.current) gutterRef.current.scrollTop = e.currentTarget.scrollTop;
  };
  // Zooms, laid out into layers so overlapping ones stack on separate rows instead of colliding.
  const zooms = layoutRegions(doc.zooms);
  const { drag, beginDrag } = useRegionDrag(zooms, dur, track, 32,
    (id, start_ms, end_ms, layer) => void onApply({ op: "update_zoom", id, start_ms, end_ms, layer }), onSel);
  const maxZoomLayer = Math.max(
    ...zooms.map((z) => z.layer),
    drag ? drag.layer : 0
  );
  const zoomRows = zooms.length ? maxZoomLayer + 1 : 0;

  // Effect regions, laid out into layers so overlapping ones stack on separate rows.
  const fx = layoutRegions(doc.effects);
  const eff = useRegionDrag(fx, dur, track, 32,
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
  const lay = useRegionDrag(layouts, dur, track, 32,
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

  // Audio row count mirrors AudioTrack's own render-or-null rule (loading -> always 2 shimmers;
  // resolved -> one row per source that actually exists) so the gutter's Audio label slot is
  // exactly as tall as however many rows actually render below it.
  const audioRows = !wavesReady ? 2 : (waves.system ? 1 : 0) + (waves.mic ? 1 : 0);
  // One entry per visible lane, in render order - mapped TWICE below (gutter label, track body)
  // from this SAME array, so the two columns can never drift out of sync with each other.
  const lanes: { key: string; label: string; heightPx: number; body: React.ReactNode }[] = [];
  if (zoomRows > 0) lanes.push({ key: "zoom", label: "Zoom", heightPx: laneHeight(zoomRows, ROW_H),
    body: <RegionRows rows={zoomRows} regions={zooms} dur={dur} sel={sel} rowClass="e-zoomrow" blkClass="e-zblk"
      dragState={drag} beginDrag={beginDrag} renderLabel={(z) => <><IconZoomIn size={12} />{z.scale.toFixed(1)}x</>} /> });
  if (fxRows > 0) lanes.push({ key: "fx", label: "FX", heightPx: laneHeight(fxRows, ROW_H),
    body: <RegionRows rows={fxRows} regions={fx} dur={dur} sel={sel} rowClass="e-fxrow" blkClass="e-fxblk"
      dragState={eff.drag} beginDrag={eff.beginDrag} renderLabel={() => <><IconBulb size={12} />Spotlight</>} /> });
  if (layRows > 0) lanes.push({ key: "layout", label: "Layout", heightPx: laneHeight(layRows, ROW_H),
    body: <RegionRows rows={layRows} regions={layouts} dur={dur} sel={sel} rowClass="e-layrow" blkClass="e-layblk"
      dragState={lay.drag} beginDrag={lay.beginDrag} renderLabel={(l) => <><IconAspectRatio size={12} />{prettyLayout(l.layout)}</>}
      // Static gradient ramps at each end, sized to the segment's own in/out transitions - the
      // pill reads as long as its fades actually are.
      extraStyle={(l, s, e) => ({ "--fin": `${transitionRampPct(l.transition_ms, e - s)}%`,
        "--fout": `${transitionRampPct(l.transition_out_ms, e - s)}%` } as React.CSSProperties)} /> });
  lanes.push({ key: "camera", label: "Camera", heightPx: ROW_H,
    body: <CameraLane doc={doc} dur={dur} sel={sel} onSel={onSel} onApply={onApply} track={track} /> });
  if (!wavesReady || waves.system || waves.mic) lanes.push({ key: "audio", label: "Audio", heightPx: laneHeight(audioRows, AUDIO_ROW_H),
    body: <><AudioTrack src={waves.system} kind="system" loading={!wavesReady} /><AudioTrack src={waves.mic} kind="mic" loading={!wavesReady} /></> });

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
          } else if (type === "cammove") {
            void onApply({ op: "add_camera_move", t_ms: dropMs, x: 0.5, y: 0.5, size: 0.25 });
          }
        }}
        onPointerDown={(e) => { scrubbing.current = true; e.currentTarget.setPointerCapture(e.pointerId); seekAt(e.clientX); }}
        onPointerMove={(e) => { if (scrubbing.current && e.buttons === 1) seekAt(e.clientX); }}
        onPointerUp={() => { scrubbing.current = false; }}
        onLostPointerCapture={() => { scrubbing.current = false; }}>
        <Filmstrip thumbs={thumbs} />
        {/* `.e-trackswrap` is `position: relative` so `.e-lanegutter` can sit `position: absolute`
            OUTSIDE `.e-tracks` (a true sibling, not a descendant) without taking any width away
            from it - `.e-tracks` stays exactly 100% of `.e-tlbody`'s width, so the pills' percentage
            math and `seekAt`/`useRegionDrag`'s `.e-tlbody`-based ms<->px conversion are completely
            untouched by the gutter's existence. */}
        <div className="e-trackswrap">
          <div className="e-lanegutter" ref={gutterRef}>
            {lanes.map((l) => (
              <div key={l.key} className="e-lanelabelrow" style={{ height: l.heightPx }}>
                <span className="e-lanelabel">{l.label}</span>
              </div>
            ))}
          </div>
          <div className="e-tracks" onScroll={onTracksScroll}>
            {lanes.map((l) => <div key={l.key} className="e-lanerows">{l.body}</div>)}
          </div>
        </div>
        <TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />
        <motion.div className="e-ph" initial={false} animate={{ left: `${pct}%` }}
          transition={playing ? { duration: 0 } : { type: "tween", duration: 0.12, ease: "easeOut" }}><i /></motion.div>
      </div>
    </div>
  );
}
