import { memo, useCallback, useMemo, useRef, useState } from "react";
import { motion } from "motion/react";
import { IconZoomIn, IconBulb, IconAspectRatio } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { rulerTicks } from "./time";
import { useRegionDrag } from "../hooks/useRegionDrag";
import { useRafCoalesced } from "../hooks/useRafCoalesced";
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
// (editor.css uses the same 32/22/6 numbers) - a lane's gutter label slot is always exactly as
// tall as the lane's real content, never computed twice with a chance to drift.
const ROW_H = 32, AUDIO_ROW_H = 22, GAP = 6;
const laneHeight = (rows: number, rowH: number) => (rows > 0 ? rows * rowH + (rows - 1) * GAP : 0);

// `RegionRows.renderLabel` for each lane - hoisted to module scope (rather than declared inline
// in the component body) since they're pure and close over nothing per-render: a fresh inline
// arrow every render would be a fresh prop every render, defeating `RegionRows`' `React.memo`
// even when the underlying region data hasn't changed.
const zoomLabel = (z: { scale: number }) => <><IconZoomIn size={12} />{z.scale.toFixed(1)}x</>;
const fxLabel = () => <><IconBulb size={12} />Spotlight</>;
const layoutLabel = (l: { layout: string }) => <><IconAspectRatio size={12} />{prettyLayout(l.layout)}</>;
const layoutExtraStyle = (l: { transition_ms: number; transition_out_ms: number }, s: number, e: number) =>
  ({ "--fin": `${transitionRampPct(l.transition_ms, e - s)}%`, "--fout": `${transitionRampPct(l.transition_out_ms, e - s)}%` } as React.CSSProperties);

export const Timeline = memo(function Timeline({ doc, timeMs, dur, playing, onSeek, sel, onSel, onApply, thumbs, waves, wavesReady, hasWebcam }: {
  doc: EditDoc; timeMs: number; dur: number; playing: boolean; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean;
  hasWebcam: boolean; // threaded straight to CameraLane - see its own prop doc
}) {
  const track = useRef<HTMLDivElement>(null);
  // Whether a SCRUB (a pointerdown that started on the body, not a pill/handle/keyframe bubbling
  // up) is in progress - every draggable child stops its own `pointerdown` propagation, so this
  // only ever flips true for a genuine body scrub (playhead grab head included - see below).
  const scrubbing = useRef(false);
  // Mirrors `scrubbing` as real state (Task D2) - ONLY consumed by the playhead's Motion-driven
  // glow layer below, which needs a re-render to fade in/out; the high-frequency scrub itself
  // still reads the ref, not this, so a pointermove burst never re-renders Timeline.
  const [phDragging, setPhDragging] = useState(false);
  // The label gutter (`.e-lanegutter`) is a SIBLING of `.e-tracks`, not a descendant - `.e-tracks`'
  // `overflow-y: auto` clips a nested descendant's escape attempts too. A sibling isn't clipped but
  // doesn't scroll for free either - `onTracksScroll` mirrors `.e-tracks`' scrollTop onto it.
  const gutterRef = useRef<HTMLDivElement>(null);
  const onTracksScroll = (e: React.UIEvent<HTMLDivElement>) => {
    if (gutterRef.current) gutterRef.current.scrollTop = e.currentTarget.scrollTop;
  };
  // Zooms/effects/layouts, laid out into layers so overlapping ones stack on separate rows instead
  // of colliding. Memoized on the doc's own array (not a fresh array reference every render) so
  // `useRegionDrag`'s `beginDrag` and `RegionRows`' `React.memo` stay stable across e.g. a playhead tick.
  const zooms = useMemo(() => layoutRegions(doc.zooms), [doc.zooms]);
  const fx = useMemo(() => layoutRegions(doc.effects), [doc.effects]);
  const nonScreenLayout = useMemo(() => doc.layout.filter((s) => s.layout !== "screen"), [doc.layout]);
  const layouts = useMemo(() => layoutRegions(nonScreenLayout), [nonScreenLayout]);

  // The three `onCommit` closures - `useCallback`'d (not a fresh inline arrow every render) so
  // they don't force `useRegionDrag`'s own attach/detach effect identity to churn.
  const onCommitZoom = useCallback((id: string, start_ms: number, end_ms: number, layer: number) =>
    void onApply({ op: "update_zoom", id, start_ms, end_ms, layer }), [onApply]);
  const onCommitFx = useCallback((id: string, start_ms: number, end_ms: number, layer: number) =>
    void onApply({ op: "update_effect", id, start_ms, end_ms, layer }), [onApply]);
  const onCommitLayout = useCallback((id: string, start_ms: number, end_ms: number) =>
    void onApply({ op: "update_layout_seg", id, start_ms, end_ms }), [onApply]);

  const { drag, beginDrag } = useRegionDrag(zooms, dur, track, 32, onCommitZoom, onSel);
  const maxZoomLayer = Math.max(
    ...zooms.map((z) => z.layer),
    drag ? drag.layer : 0
  );
  const zoomRows = zooms.length ? maxZoomLayer + 1 : 0;

  // Effect regions.
  const eff = useRegionDrag(fx, dur, track, 32, onCommitFx, onSel);
  const maxFxLayer = Math.max(
    ...fx.map((f) => f.layer),
    eff.drag ? eff.drag.layer : 0
  );
  const fxRows = fx.length ? maxFxLayer + 1 : 0;

  // Layout regions. No priority layer (update_layout_seg takes no `layer` - the layer arg
  // useRegionDrag reports is row-stacking only). "screen" is the empty default, meaningless as a
  // pill - hidden from the track above; only non-screen layouts show as pills.
  const lay = useRegionDrag(layouts, dur, track, 32, onCommitLayout, onSel);
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
  // Scrub coalescing: a pointermove burst can outpace the refresh rate, so `scheduleSeek` applies
  // at most the LATEST position once per frame; `flushSeek` (on release) applies a pending one now.
  const { schedule: scheduleSeek, flush: flushSeek } = useRafCoalesced(seekAt);
  const pct = dur > 0 ? (timeMs / dur) * 100 : 0;

  // Audio row count mirrors AudioTrack's own render-or-null rule: loading -> 2 shimmers, resolved
  // -> one row per source that actually exists.
  const audioRows = !wavesReady ? 2 : (waves.system ? 1 : 0) + (waves.mic ? 1 : 0);
  // Whether `sel` belongs to this lane's own regions - the gutter label brightens to --e-fg for
  // the lane owning the current selection (Task D2), dimming every other lane's label instead.
  const isSel = (regions: { id: string }[]) => sel != null && regions.some((r) => r.id === sel);
  // One entry per visible lane, in render order - mapped TWICE below (gutter label, track body)
  // from this SAME array, so the two columns can never drift out of sync with each other.
  const lanes: { key: string; label: string; heightPx: number; active: boolean; body: React.ReactNode }[] = [];
  if (zoomRows > 0) lanes.push({ key: "zoom", label: "Zoom", heightPx: laneHeight(zoomRows, ROW_H), active: isSel(zooms),
    body: <RegionRows rows={zoomRows} regions={zooms} dur={dur} sel={sel} rowClass="e-zoomrow" blkClass="e-zblk"
      dragState={drag} beginDrag={beginDrag} renderLabel={zoomLabel} /> });
  if (fxRows > 0) lanes.push({ key: "fx", label: "FX", heightPx: laneHeight(fxRows, ROW_H), active: isSel(fx),
    body: <RegionRows rows={fxRows} regions={fx} dur={dur} sel={sel} rowClass="e-fxrow" blkClass="e-fxblk"
      dragState={eff.drag} beginDrag={eff.beginDrag} renderLabel={fxLabel} /> });
  if (layRows > 0) lanes.push({ key: "layout", label: "Layout", heightPx: laneHeight(layRows, ROW_H), active: isSel(layouts),
    // Fade ramps: gradient widths sized to the segment's own in/out transitions.
    body: <RegionRows rows={layRows} regions={layouts} dur={dur} sel={sel} rowClass="e-layrow" blkClass="e-layblk"
      dragState={lay.drag} beginDrag={lay.beginDrag} renderLabel={layoutLabel} extraStyle={layoutExtraStyle} /> });
  lanes.push({ key: "camera", label: "Camera", heightPx: ROW_H, active: isSel(doc.camera_moves),
    body: <CameraLane doc={doc} dur={dur} sel={sel} onSel={onSel} onApply={onApply} track={track} hasWebcam={hasWebcam} /> });
  if (!wavesReady || waves.system || waves.mic) lanes.push({ key: "audio", label: "Audio", heightPx: laneHeight(audioRows, AUDIO_ROW_H), active: false,
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
        onPointerDown={(e) => { scrubbing.current = true; setPhDragging(true); e.currentTarget.setPointerCapture(e.pointerId); seekAt(e.clientX); }}
        onPointerMove={(e) => { if (scrubbing.current && e.buttons === 1) scheduleSeek(e.clientX); }}
        onPointerUp={() => { scrubbing.current = false; setPhDragging(false); flushSeek(); }}
        onLostPointerCapture={() => { scrubbing.current = false; setPhDragging(false); flushSeek(); }}>
        <Filmstrip thumbs={thumbs} />
        {/* `.e-trackswrap` is `position: relative` so `.e-lanegutter` sits `position: absolute`
            OUTSIDE `.e-tracks`, taking no width from it - the pills' %/ms<->px math is untouched. */}
        <div className="e-trackswrap">
          <div className="e-lanegutter" ref={gutterRef}>
            {lanes.map((l) => (
              <div key={l.key} className="e-lanelabelrow" style={{ height: l.heightPx }}>
                <span className={`e-lanelabel${l.active ? " on" : ""}`}>{l.label}</span>
              </div>
            ))}
          </div>
          <div className="e-tracks" onScroll={onTracksScroll}>
            {/* Band shade keys off this lane's INDEX in the already-filtered array, not its type -
                a type->shade mapping put two same-shade lanes adjacent whenever only one of
                {fx, layout} was present. */}
            {lanes.map((l, i) => <div key={l.key} className={`e-lanerows e-band-${i % 2 ? "b" : "a"}`}>{l.body}</div>)}
          </div>
        </div>
        <TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />
        {/* Playhead (Task D2): the line's own left% still tweens via Motion (unchanged); its two
            children are ALSO Motion-owned - the glow's opacity crossfades on `phDragging` (state,
            not the high-frequency `scrubbing` ref) and the grab head's hover scale is a spring,
            not a CSS transition, per the motion-language rule for stateful interaction. */}
        <motion.div className="e-ph" initial={false} animate={{ left: `${pct}%` }}
          transition={playing ? { duration: 0 } : { type: "tween", duration: 0.12, ease: "easeOut" }}>
          <motion.i className="e-ph-glow" initial={false} animate={{ opacity: phDragging ? 1 : 0 }} transition={{ duration: 0.15, ease: "easeOut" }} />
          <motion.i className="e-ph-head" whileHover={{ scale: 1.15 }} transition={{ type: "spring", stiffness: 420, damping: 22 }} />
        </motion.div>
      </div>
    </div>
  );
});
