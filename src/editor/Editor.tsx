import { useRef, useState } from "react";
import { saveEdit, aiPlan, exportProject, applyEditOp, fileSrc, DEFAULT_PROXY_HEIGHT } from "../lib/ipc";
import type { EditDoc, EditOp } from "../lib/edit";
import { resolveTrim } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import { TopBar } from "./shell/TopBar";
import { ExportDialog } from "./shell/ExportDialog";
import { ResizeEdges } from "./controls/ResizeEdges";
import { Rail, type Tab } from "./shell/Rail";
import { EditorPanels } from "./EditorPanels";
import { Stage } from "./stage/Stage";
import { Transport } from "./stage/Transport";
import { Timeline } from "./timeline/Timeline";
import { useEditorData } from "./hooks/useEditorData";
import { useEditorKeymap } from "./hooks/useEditorKeymap";
import { useMoveModeGuard } from "./hooks/useMoveModeGuard";
import { useTrimActions } from "./hooks/useTrimActions";
import { useEditHistory } from "./hooks/useEditHistory";
import "./editor.css";

/** The post-record editor. The preview plays the recording natively (Stage) and applies
 *  the exact camera curve (camera_track) as a transform - smooth 60fps. Edits go through
 *  the Edit API; `rev` bumps so the camera curve refetches and reflects them. */
export function Editor({ folder, onClose }: { folder: string; onClose: () => void }) {
  const [timeMs, setTimeMs] = useState(0);
  const [rev, setRev] = useState(0);
  const [tab, setTab] = useState<Tab>("ai");
  const [sel, setSel] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [aiError, setAiError] = useState<string | null>(null); // last AI auto-edit failure, shown in the panel
  const [aiLog, setAiLog] = useState<string[]>([]); // agentic director's live "what I did" narration
  const [vidDurMs, setVidDurMs] = useState(0);
  const [quality, setQuality] = useState(DEFAULT_PROXY_HEIGHT);
  const [muted, setMuted] = useState(false);
  const [volume, setVolume] = useState(100); // 0..100 preview-audio volume (transport slider)
  const [showExportDialog, setShowExportDialog] = useState(false);
  // The UNSAVED Move-mode webcam pose: dragging the PiP updates it live (preview only), the Camera
  // panel's Update/Add button saves it as a keyframe, and Stage discards it when the playhead moves.
  // Shared here so both Stage (drag) and CameraPanel (save button) see the same draft.
  const camDraftRef = useRef<CamPose | null>(null);

  const {
    doc, setDoc, track, layout, layoutPresets, clicks, bgUrl, cursorSpr, cursorKnd,
    thumbs, waves, audioUrl, srcUrl, playing, setPlaying, exporting, setExporting, pct, setPct,
    exportDone, setExportDone, exportError, setExportError,
  } = useEditorData(folder, rev, quality);

  const dur = vidDurMs > 0 ? vidDurMs : (doc?.trim.out_ms ?? 0);
  const proj = folder.split(/[\\/]/).pop() ?? "project";
  const { record, undo, redo, canUndo, canRedo } = useEditHistory(folder, doc, setDoc, () => setRev((r) => r + 1));

  // Clamp playback to the trim's out point (not just the clip end), so pressing play never runs
  // past a trimmed-out tail; scrubbing the timeline itself is unrestricted (resolveTrim's inMs/
  // outMs mirror the export gate exactly, via the same doc.trim the backend reads).
  const onTime = (ms: number) => {
    const { outMs } = resolveTrim(doc?.trim ?? { in_ms: 0, out_ms: 0 }, dur);
    // At the trim-out, park the playhead exactly on the boundary - not in the frame or two of
    // overshoot the rAF loop reports before the pause lands - so it doesn't sit past the line.
    if (dur > 0 && ms >= outMs) { setTimeMs(outMs); setPlaying(false); }
    else setTimeMs(ms);
  };

  const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
    if (doc) record(doc); // snapshot the pre-edit doc for undo
    try {
      const d = await applyEditOp(folder, op); setDoc(d);
      if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
      return d;
    } catch { return null; }
  };
  const { moveMode, requestMoveMode, moveOffDialog } = useMoveModeGuard(doc, applyOp);
  const { onTrimIn, onTrimOut, onResetTrim } = useTrimActions(doc, timeMs, dur, applyOp);
  const saveDocSettings = async (nextSettings: EditDoc["settings"]) => {
    if (!doc) return;
    record(doc); // snapshot for undo
    const newDoc = { ...doc, settings: nextSettings };
    setDoc(newDoc);
    await saveEdit(folder, newDoc);
    setRev((r) => r + 1);
  };
  const addZoom = async () => {
    const d = await applyOp({ op: "add_zoom", at_ms: Math.round(timeMs), dur_ms: 2000 });
    if (d && d.zooms.length) setSel(d.zooms[d.zooms.length - 1].id);
  };
  const addSpotlight = async () => {
    const d = await applyOp({ op: "add_effect", kind: "spotlight", start_ms: Math.round(timeMs), end_ms: Math.round(timeMs) + 2000 });
    if (d && d.effects.length) setSel(d.effects[d.effects.length - 1].id);
  };
  const addCameraMove = async () => {
    const d = await applyOp({ op: "add_camera_move", t_ms: Math.round(timeMs), x: 0.5, y: 0.5, size: 0.25 });
    if (d && d.camera_moves.length) setSel(d.camera_moves[d.camera_moves.length - 1].id);
  };

  useEditorKeymap({ sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight });

  const zoomAt = async (x: number, y: number) => {
    setPlaying(false);
    const d = await applyOp({ op: "add_zoom_full", at_ms: Math.round(timeMs), dur_ms: 2000, scale: 2.5 });
    if (d && d.zooms.length) {
      const id = d.zooms[d.zooms.length - 1].id;
      await applyOp({ op: "update_zoom", id, target: { fixed: { x, y } } });
      setSel(id);
    }
  };

  // Agentic AI director: fetch the whole plan in one LLM call, then REVEAL the edits one-at-a-time
  // (apply + narrate + scrub the preview to each zoom, ~460ms apart) so it reads like a live agent
  // editing the panels. `record(doc)` once => the whole pass is a single undo. Failures surface in
  // the panel (e.g. "Could not reach Ollama ... Try: ollama serve") instead of a silent no-op.
  const onRun = async () => {
    if (!doc) return;
    record(doc);
    setRunning(true); setAiError(null); setAiLog([]);
    try {
      const steps = await aiPlan(folder, doc.settings.ai_model || undefined);
      for (const step of steps) {
        const d = await applyEditOp(folder, step.op); // raw apply - no per-step undo record
        setDoc(d); setRev((r) => r + 1);
        setAiLog((l) => [...l, step.label]);
        if (step.op.op === "add_zoom_full") { setPlaying(false); setTimeMs(step.op.at_ms); }
        await new Promise((res) => setTimeout(res, 460));
      }
      const zoomN = steps.filter((s) => s.op.op === "add_zoom_full").length;
      setAiLog((l) => [...l, `✓ Done · ${zoomN} zoom${zoomN === 1 ? "" : "s"}`]);
    } catch (e) { setAiError(String(e)); }
    finally { setRunning(false); }
  };

  if (!doc) return <div className="editor"><div className="e-stage-empty" style={{ margin: "auto" }}>Loading edit...</div></div>;

  const trimRange = resolveTrim(doc.trim, dur);
  const trimmed = trimRange.inMs > 0 || trimRange.outMs < dur;

  return (
    <div className="editor">
      <ResizeEdges />
      <TopBar proj={proj} exporting={exporting} pct={pct} onOpenExport={() => setShowExportDialog(true)} onClose={onClose}
        onUndo={() => void undo()} onRedo={() => void redo()} canUndo={canUndo} canRedo={canRedo} />
      <div className="e-body">
        <Rail tab={tab} onTab={(t) => { setSel(null); setTab(t); }} />
        <EditorPanels doc={doc} sel={sel} tab={tab} dur={dur} setSel={setSel} setTab={setTab}
          timeMs={timeMs} running={running} aiError={aiError} aiLog={aiLog} onRun={onRun} applyOp={applyOp} saveDocSettings={saveDocSettings}
          moveMode={moveMode} requestMoveMode={requestMoveMode} camDraftRef={camDraftRef}
          addZoom={addZoom} addSpotlight={addSpotlight} addCameraMove={addCameraMove} />
        <Stage src={srcUrl} webcamSrc={fileSrc(`${folder}\\webcam.webm`)} track={track} layout={layout} layoutPresets={layoutPresets} layoutSegs={doc.layout} cameraMoves={doc.camera_moves} zooms={doc.zooms} zoomSettings={doc.settings.zoom} clicks={clicks} bgUrl={bgUrl} cursorSprites={cursorSpr} cursorKinds={cursorKnd} cursor={doc.settings.cursor} effects={doc.effects} clickfx={doc.settings.clickfx} audioSrc={audioUrl} muted={muted} volume={volume / 100} timeMs={timeMs} playing={playing} moveMode={moveMode} camDraftRef={camDraftRef} onTime={onTime} onDuration={setVidDurMs} onZoomAt={zoomAt} />
      </div>
      <Transport
        timeMs={timeMs}
        dur={dur}
        playing={playing}
        onPlay={() => setPlaying((p) => {
          // Starting playback outside the trim range snaps forward to the trim-in point first,
          // so play never starts inside a dimmed (trimmed-out) region.
          if (!p && (timeMs < trimRange.inMs || timeMs >= trimRange.outMs)) setTimeMs(trimRange.inMs);
          return !p;
        })}
        onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }}
        onAddZoom={addZoom}
        onAutoedit={onRun}
        trimmed={trimmed}
        onTrimIn={onTrimIn}
        onTrimOut={onTrimOut}
        onResetTrim={onResetTrim}
        aspect={doc.aspect}
        onAspect={(aspect) => { void applyOp({ op: "set_aspect", aspect }); }}
        quality={quality}
        onQuality={() => setQuality((q) => (q === 480 ? 720 : q === 720 ? 1080 : 480))}
        muted={muted}
        onMute={() => setMuted((m) => !m)}
        volume={volume}
        onVolume={setVolume}
      />
      <Timeline doc={doc} timeMs={timeMs} dur={dur} playing={playing} onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }} sel={sel} onSel={setSel} onApply={applyOp} thumbs={thumbs} waves={waves} />
      {moveOffDialog}
      <ExportDialog open={showExportDialog} exporting={exporting} pct={pct} done={exportDone} error={exportError} onClose={() => setShowExportDialog(false)} onReset={() => { setExportDone(false); setExportError(null); }} onExport={(settings) => { setExportDone(false); setExportError(null); setExporting(true); setPct(0); void exportProject(folder, settings); }} />
    </div>
  );
}
