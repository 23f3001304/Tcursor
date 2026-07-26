import { useRef, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { saveEdit, aiAutoedit, exportProject, applyEditOp, fileSrc, DEFAULT_PROXY_HEIGHT } from "../lib/ipc";
import type { EditDoc, EditOp } from "../lib/edit";
import { resolveTrim } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import { TopBar } from "./shell/TopBar";
import { ExportDialog } from "./shell/ExportDialog";
import { ResizeEdges } from "./controls/ResizeEdges";
import { Rail, type Tab } from "./shell/Rail";
import { AiPanel } from "./panels/AiPanel";
import { ZoomInspector } from "./inspectors/ZoomInspector";
import { EffectInspector } from "./inspectors/EffectInspector";
import { LayoutInspector } from "./inspectors/LayoutInspector";
import { CameraMoveInspector } from "./inspectors/CameraMoveInspector";
import { BackgroundPanel } from "./panels/BackgroundPanel";
import { CursorPanel } from "./panels/CursorPanel";
import { CameraPanel } from "./panels/CameraPanel";
import { CaptionsPanel } from "./panels/CaptionsPanel";
import { AudioPanel } from "./panels/AudioPanel";
import { EffectsPanel } from "./panels/EffectsPanel";
import { Stage } from "./stage/Stage";
import { Transport } from "./stage/Transport";
import { Timeline } from "./timeline/Timeline";
import { useEditorData } from "./hooks/useEditorData";
import { useEditorKeymap } from "./hooks/useEditorKeymap";
import { useMoveModeGuard } from "./hooks/useMoveModeGuard";
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
  const [vidDurMs, setVidDurMs] = useState(0);
  const [quality, setQuality] = useState(DEFAULT_PROXY_HEIGHT);
  const [muted, setMuted] = useState(false);
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

  // Clamp playback to the trim's out point (not just the clip end), so pressing play never runs
  // past a trimmed-out tail; scrubbing the timeline itself is unrestricted (resolveTrim's inMs/
  // outMs mirror the export gate exactly, via the same doc.trim the backend reads).
  const onTime = (ms: number) => {
    setTimeMs(ms);
    const { outMs } = resolveTrim(doc?.trim ?? { in_ms: 0, out_ms: 0 }, dur);
    if (dur > 0 && ms >= outMs) setPlaying(false);
  };

  const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
    try {
      const d = await applyEditOp(folder, op); setDoc(d);
      if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
      return d;
    } catch { return null; }
  };
  const { moveMode, requestMoveMode, moveOffDialog } = useMoveModeGuard(doc, applyOp);
  const saveDocSettings = async (nextSettings: EditDoc["settings"]) => {
    if (!doc) return;
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

  const onRun = async () => {
    setRunning(true);
    try { setDoc(await aiAutoedit(folder, doc?.settings.ai_model || undefined)); setRev((r) => r + 1); }
    catch { /* Ollama down */ } finally { setRunning(false); }
  };

  if (!doc) return <div className="editor"><div className="e-stage-empty" style={{ margin: "auto" }}>Loading edit...</div></div>;

  const selZoom = doc.zooms.find((z) => z.id === sel) ?? null;
  const selEffect = doc.effects.find((e) => e.id === sel) ?? null;
  const selLayout = doc.layout.find((l) => l.id === sel) ?? null;
  const selCamMove = doc.camera_moves.find((m) => m.id === sel) ?? null;
  const trimRange = resolveTrim(doc.trim, dur);
  const trimmed = trimRange.inMs > 0 || trimRange.outMs < dur;

  return (
    <div className="editor">
      <ResizeEdges />
      <TopBar proj={proj} exporting={exporting} pct={pct} onOpenExport={() => setShowExportDialog(true)} onClose={onClose} />
      <div className="e-body">
        <Rail tab={tab} onTab={(t) => { setSel(null); setTab(t); }} />
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div key={sel ?? tab} className="e-panel-slot"
            initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
            {selZoom ? (
              <ZoomInspector zoom={selZoom} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
            ) : selEffect ? (
              <EffectInspector effect={selEffect} dur={dur} settings={doc.settings} onApply={applyOp}
                onDimCamera={(v) => saveDocSettings({ ...doc.settings, clickfx: { ...doc.settings.clickfx, spotlight_dim_camera: v } })}
                onClose={() => setSel(null)} />
            ) : selLayout ? (
              <LayoutInspector seg={selLayout} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
            ) : selCamMove ? (
              <CameraMoveInspector move={selCamMove} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
            ) : tab === "ai" ? (
              <AiPanel running={running} onRun={onRun} model={doc.settings.ai_model}
                onChangeModel={(v) => saveDocSettings({ ...doc.settings, ai_model: v })} />
            ) : tab === "background" ? (
              <BackgroundPanel doc={doc} onSaveSettings={saveDocSettings} onClose={() => setTab("ai")} />
            ) : tab === "cursor" ? (
              <CursorPanel settings={doc.settings.cursor} onChange={(cursor) => saveDocSettings({ ...doc.settings, cursor })} onClose={() => setTab("ai")} />
            ) : tab === "camera" ? (
              <CameraPanel settings={doc.settings.appearance} onChange={(appearance) => saveDocSettings({ ...doc.settings, appearance })} onClose={() => setTab("ai")}
                doc={doc} timeMs={timeMs} applyOp={applyOp} moveMode={moveMode} onMoveModeChange={requestMoveMode} camDraftRef={camDraftRef} />
            ) : tab === "captions" ? (
              <CaptionsPanel settings={doc.settings.clickfx} onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })} onClose={() => setTab("ai")} />
            ) : tab === "audio" ? (
              <AudioPanel offsetMs={doc.settings.audio_offset_ms} onChangeOffset={(v) => saveDocSettings({ ...doc.settings, audio_offset_ms: v })}
                micVol={doc.settings.audio_mic_volume} onChangeMicVol={(v) => saveDocSettings({ ...doc.settings, audio_mic_volume: v })}
                sysVol={doc.settings.audio_sys_volume} onChangeSysVol={(v) => saveDocSettings({ ...doc.settings, audio_sys_volume: v })}
                onClose={() => setTab("ai")} />
            ) : tab === "effects" ? (
              <EffectsPanel settings={doc.settings.clickfx} onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })} onClose={() => setTab("ai")} onAddZoom={addZoom} onAddSpotlight={addSpotlight} onAddLayout={async () => { await applyOp({ op: "add_layout_seg", at_ms: Math.round(timeMs), dur_ms: 2000, layout: "camera" }); }} onAddCameraMove={addCameraMove} />
            ) : (
              <div className="e-panel">
                <h2 style={{ textTransform: "capitalize" }}>{tab}</h2>
                <p className="e-lede">{tab} settings land here next.</p>
              </div>
            )}
          </motion.div>
        </AnimatePresence>
        <Stage src={srcUrl} webcamSrc={fileSrc(`${folder}\\webcam.webm`)} track={track} layout={layout} layoutPresets={layoutPresets} layoutSegs={doc.layout} cameraMoves={doc.camera_moves} zooms={doc.zooms} zoomSettings={doc.settings.zoom} clicks={clicks} bgUrl={bgUrl} cursorSprites={cursorSpr} cursorKinds={cursorKnd} cursor={doc.settings.cursor} effects={doc.effects} clickfx={doc.settings.clickfx} audioSrc={audioUrl} muted={muted} timeMs={timeMs} playing={playing} moveMode={moveMode} camDraftRef={camDraftRef} onTime={onTime} onDuration={setVidDurMs} onZoomAt={zoomAt} />
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
        onSplit={async () => {
          await applyOp({ op: "add_cut", start_ms: Math.round(timeMs), end_ms: Math.round(timeMs) + 1000 });
        }}
        trimmed={trimmed}
        onResetTrim={async () => { await applyOp({ op: "set_trim", in_ms: 0, out_ms: 0 }); }}
        aspect={doc.aspect}
        onAspect={(aspect) => { void applyOp({ op: "set_aspect", aspect }); }}
        quality={quality}
        onQuality={() => setQuality((q) => (q === 480 ? 720 : q === 720 ? 1080 : 480))}
        muted={muted}
        onMute={() => setMuted((m) => !m)}
      />
      <Timeline doc={doc} timeMs={timeMs} dur={dur} playing={playing} onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }} sel={sel} onSel={setSel} onApply={applyOp} thumbs={thumbs} waves={waves} />
      {moveOffDialog}
      <ExportDialog open={showExportDialog} exporting={exporting} pct={pct} done={exportDone} error={exportError} onClose={() => setShowExportDialog(false)} onReset={() => { setExportDone(false); setExportError(null); }} onExport={(settings) => { setExportDone(false); setExportError(null); setExporting(true); setPct(0); void exportProject(folder, settings); }} />
    </div>
  );
}
