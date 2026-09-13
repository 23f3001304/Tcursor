import { memo, useCallback, useMemo, useRef, useState } from "react";
import { IconZoomIn, IconBulb } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../lib/edit";
import type { LayoutPresets } from "../../lib/ipc";
import { useRegionDrag } from "../hooks/useRegionDrag";
import { layoutRegions } from "./layers";
import { useLayoutLaneRegions, layoutLabel, layoutExtraStyle } from "./layoutLane";
import { Filmstrip } from "./Filmstrip";
import { AudioTrack } from "./AudioTrack";
import { CameraLane } from "./CameraLane";
import { TimeLane } from "./TimeLane";
import { CutOverlay } from "./CutOverlay";
import { TrimOverlay } from "./TrimOverlay";
import { RegionRows } from "./RegionRows";
import { Playhead } from "./Playhead";
import { Ruler, RangeOverlay, useSeek } from "./timelineRuler";
import type { Range } from "./useRangeSelect";

/** Multi-track timeline (Filmora-style): an adaptive ruler (`timelineRuler.tsx`), a filmstrip clip,
 *  and a scrolling stack of tracks - Time, zoom, FX, layout (pills drag/resize via useRegionDrag),
 *  camera and the two audio waveforms - with a playhead spanning the clip. Selecting a pill (or a
 *  cut) opens its inspector; add-zoom lives in the transport. The playhead lives inside the body so
 *  it stays aligned with the ruler ticks + pills regardless of the timeline's outer padding. */

// Row-height constants shared by the lane label gutter and `.e-tracks`' own natural row layout
// (editor.css uses the same 32/22/6 numbers) - a lane's gutter label slot is always exactly as
// tall as the lane's real content, never computed twice with a chance to drift.
const ROW_H = 32, AUDIO_ROW_H = 22, GAP = 6;
const laneHeight = (rows: number, rowH: number) => (rows > 0 ? rows * rowH + (rows - 1) * GAP : 0);

// `RegionRows.renderLabel` for the zoom/FX lanes - module scope, not inline in the body: they are
// pure and close over nothing per-render, and a fresh arrow every render would be a fresh prop
// every render, defeating `RegionRows`' memo even with unchanged region data. The layout lane's
// own `layoutLabel`/`layoutExtraStyle` live in `layoutLane.tsx`, the Time lane's in `TimeLane.tsx`.
const zoomLabel = (z: { scale: number }) => <><IconZoomIn size={12} />{z.scale.toFixed(1)}x</>;
const fxLabel = () => <><IconBulb size={12} />Spotlight</>;

export const Timeline = memo(function Timeline({ doc, timeMs, dur, playing, onSeek, sel, onSel, onApply, thumbs, waves, wavesReady, hasWebcam, layoutPresets, range, setRange }: {
  doc: EditDoc; timeMs: number; dur: number; playing: boolean; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean;
  hasWebcam: boolean; // threaded straight to CameraLane - see its own prop doc
  layoutPresets: LayoutPresets | null; // T34 L4: the layout pill's thumbnail source (layoutLane.tsx)
  range: Range | null; setRange: (r: Range | null) => void; // the ruler's Shift+drag selection
}) {
  const track = useRef<HTMLDivElement>(null);
  // Whether a SCRUB (a pointerdown on the body, not a pill/handle/keyframe bubbling up) is in
  // progress - every draggable child stops its own `pointerdown`, so this only ever flips true for
  // a genuine body scrub (playhead grab head included - see below).
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
  const layouts = useLayoutLaneRegions(doc.layout, layoutPresets);

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

  // `seekAt` and its rAF coalescing live in `timelineRuler.tsx`: one x-to-ms mapping, measured off
  // the same element, shared by the ruler's own drag and the track body's.
  const { seekAt, scheduleSeek, flushSeek } = useSeek(dur, track, onSeek);
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
  // Time lane FIRST (nearest the filmstrip): cuts and speed spans are the clip's own structure, above
  // the decorations every other lane holds. Only once one exists - see TimeLane.md.
  if (doc.cuts.length || doc.speed.length) lanes.push({ key: "time", label: "Time", heightPx: ROW_H, active: isSel(doc.speed),
    body: <TimeLane doc={doc} dur={dur} sel={sel} onSel={onSel} onApply={onApply} track={track} /> });
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
      <Ruler dur={dur} trackRef={track} onSeek={onSeek} range={range} setRange={setRange} />
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
          {/* Cuts cross the whole stack, outside `.e-tracks` - CutOverlay.md says why. */}
          <CutOverlay cuts={doc.cuts} dur={dur} sel={sel} onSel={onSel} />
        </div>
        <RangeOverlay range={range} dur={dur} />
        <TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />
        {/* Playhead: line, glow and drag ripple all live in `Playhead.tsx` - `phDragging` is
            state, not the high-frequency `scrubbing` ref, so a pointermove burst never re-renders
            this component just to light the glow. */}
        <Playhead pct={pct} playing={playing} dragging={phDragging} />
      </div>
    </div>
  );
});
