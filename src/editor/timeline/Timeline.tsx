import { memo, useRef, useState } from "react";
import type { EditDoc, EditOp, TextKind } from "../../shared/edit";
import type { LayoutPresets } from "../../shared/ipc";
import type { TimeMap } from "../../shared/math/remap";
import { useTimelineLanes } from "./useTimelineLanes";
import { Filmstrip } from "./lanes/Filmstrip";
import { CutOverlay } from "./lanes/CutOverlay";
import { TrimOverlay } from "./lanes/TrimOverlay";
import { Playhead } from "./lanes/Playhead";
import { Ruler, RangeOverlay, useSeek } from "./lanes/TimelineRuler";
import type { Range } from "./useRangeSelect";
import "./timeline.css";

export const Timeline = memo(function Timeline({
  doc,
  timeMs,
  dur,
  playing,
  onSeek,
  sel,
  onSel,
  onApply,
  thumbs,
  waves,
  wavesReady,
  hasWebcam,
  layoutPresets,
  range,
  setRange,
  map,
}: {
  doc: EditDoc;
  timeMs: number;
  dur: number;
  playing: boolean;
  onSeek: (ms: number) => void;
  sel: string | null;
  onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[];
  waves: { system: string; mic: string };
  wavesReady: boolean;
  hasWebcam: boolean;
  layoutPresets: LayoutPresets | null;
  range: Range | null;
  setRange: (r: Range | null) => void;
  map: TimeMap;
}) {
  const track = useRef<HTMLDivElement>(null);
  const scrubbing = useRef(false);
  const [phDragging, setPhDragging] = useState(false);
  const gutterRef = useRef<HTMLDivElement>(null);
  const onTracksScroll = (e: React.UIEvent<HTMLDivElement>) => {
    if (gutterRef.current) gutterRef.current.scrollTop = e.currentTarget.scrollTop;
  };

  const lanes = useTimelineLanes({
    doc,
    dur,
    sel,
    onSel,
    onApply,
    track,
    waves,
    wavesReady,
    hasWebcam,
    layoutPresets,
    map,
  });

  const { seekAt, scheduleSeek, flushSeek } = useSeek(dur, track, onSeek);
  const pct = dur > 0 ? (timeMs / dur) * 100 : 0;

  return (
    <div className="e-timeline">
      <Ruler dur={dur} trackRef={track} onSeek={onSeek} range={range} setRange={setRange} />
      <div
        className="e-tlbody"
        ref={track}
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
        }}
        onDrop={(e) => {
          e.preventDefault();
          const type = e.dataTransfer.getData("text/plain");
          const el = track.current;
          if (!el) return;
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
          } else if (type === "blur" || type === "pixelate" || type === "highlight") {
            void onApply({ op: "add_effect", kind: type, start_ms: dropMs, end_ms: dropMs + 3000 });
          } else if (type.startsWith("text:")) {
            void onApply({
              op: "add_text",
              at_ms: dropMs,
              dur_ms: 3000,
              kind: type.slice(5) as TextKind,
            });
          }
        }}
        onPointerDown={(e) => {
          scrubbing.current = true;
          setPhDragging(true);
          e.currentTarget.setPointerCapture(e.pointerId);
          seekAt(e.clientX);
        }}
        onPointerMove={(e) => {
          if (scrubbing.current && e.buttons === 1) scheduleSeek(e.clientX);
        }}
        onPointerUp={() => {
          scrubbing.current = false;
          setPhDragging(false);
          flushSeek();
        }}
        onLostPointerCapture={() => {
          scrubbing.current = false;
          setPhDragging(false);
          flushSeek();
        }}
      >
        <Filmstrip thumbs={thumbs} />
        <div className="e-trackswrap">
          <div className="e-lanegutter" ref={gutterRef}>
            {lanes.map((l) => (
              <div key={l.key} className={`e-lanelabelrow e-lane-${l.key}`} style={{ height: l.heightPx }}>
                <span className={`e-lanelabel${l.active ? " on" : ""}`}>{l.label}</span>
              </div>
            ))}
          </div>
          <div className="e-tracks" data-ui-fx="off" onScroll={onTracksScroll}>
            {lanes.map((l) => (
              <div key={l.key} className={`e-lanerows e-lane-${l.key}`}>
                {l.body}
              </div>
            ))}
          </div>
          <CutOverlay cuts={doc.cuts} dur={dur} sel={sel} onSel={onSel} />
        </div>
        <RangeOverlay range={range} dur={dur} />
        <TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />
        <Playhead pct={pct} playing={playing} dragging={phDragging} />
      </div>
    </div>
  );
});
