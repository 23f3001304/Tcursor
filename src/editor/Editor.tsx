import { useRef, useState } from "react";
import { exportProject, applyEditOp, fileSrc, DEFAULT_PROXY_HEIGHT } from "../lib/ipc";
import type { EditDoc, EditOp } from "../lib/edit";
import type { MarkState } from "../lib/brandWave";
import { resolveTrim } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import { TopBar } from "./shell/TopBar";
import { ExportDialog } from "./shell/ExportDialog";
import { ShortcutsOverlay } from "./shell/ShortcutsOverlay";
import { EditorSettingsDialog } from "./shell/settings/EditorSettingsDialog";
import { Toast, useUndoToast } from "./shell/Toast";
import { ResizeEdges } from "./controls/ResizeEdges";
import { Rail, type Tab } from "./shell/Rail";
import { EditorPanels } from "./EditorPanels";
import { Stage } from "./stage/Stage";
import { zoomTargetPoint } from "./stage/zoomTargetMapper";
import { Transport } from "./stage/Transport";
import { Timeline } from "./timeline/Timeline";
import { useEditorData } from "./hooks/useEditorData";
import { useEditorKeymap } from "./hooks/useEditorKeymap";
import { useMoveModeGuard } from "./hooks/useMoveModeGuard";
import { useTrimActions } from "./hooks/useTrimActions";
import { useEditHistory } from "./hooks/useEditHistory";
import { useDocSettings } from "./hooks/useDocSettings";
import { createQueue } from "./hooks/opQueue";
import { useDirector } from "./director/useDirector";
import { useTimelineActions } from "./hooks/useTimelineActions";
import { DirectorOverlay } from "./director/DirectorOverlay";
import "./editor.css";

/** The post-record editor. The preview plays the recording natively (Stage) and applies
 *  the exact camera curve (camera_track) as a transform - smooth 60fps. Edits go through
 *  the Edit API; `rev` bumps so the camera curve refetches and reflects them. */
export function Editor({ folder, onClose }: { folder: string; onClose: () => void }) {
  const [timeMs, setTimeMs] = useState(0);
  const [rev, setRev] = useState(0);
  const [tab, setTab] = useState<Tab>("ai");
  const [sel, setSel] = useState<string | null>(null);
  // On-stage aim mode for a Region zoom. Mutually exclusive with the camera Move mode - both claim
  // the same pointer on the same canvas - so entering Move exits aiming, and aiming stays
  // suppressed (not cancelled) while Move is on. See `aimMode` below.
  const [aimOn, setAimOn] = useState(false);
  const [running, setRunning] = useState(false);
  const [aiError, setAiError] = useState<string | null>(null); // last AI auto-edit failure, shown in the panel
  const [aiLog, setAiLog] = useState<string[]>([]); // agentic director's live "what I did" narration
  const [vidDurMs, setVidDurMs] = useState(0);
  const [quality, setQuality] = useState(DEFAULT_PROXY_HEIGHT);
  const [muted, setMuted] = useState(false);
  const [volume, setVolume] = useState(100); // 0..100 preview-audio volume (transport slider)
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  // The UNSAVED Move-mode webcam pose: dragging the PiP updates it live (preview only), the Camera
  // panel's Update/Add button saves it as a keyframe, and Stage discards it when the playhead moves.
  // Shared here so both Stage (drag) and CameraPanel (save button) see the same draft.
  const camDraftRef = useRef<CamPose | null>(null);
  // Fed to useEditHistory's onSwap below - a minimal "Undid"/"Redid" pill for keyboard-triggered undo/redo.
  const { msg: toast, onSwap, dismiss: dismissToast } = useUndoToast();
  // Serializes every EditDoc mutation (applyOp/onRun/undo/redo) through one promise chain (see
  // opQueue.ts + Editor.md) so none of them can race a still-in-flight one.
  const enqueue = useRef(createQueue()).current;

  const {
    doc, setDoc, track, layout, layoutPresets, clicks, bgUrl, cursorSpr, cursorKnd, osCursor,
    thumbs, waves, wavesReady, audioUrl, srcUrl, playing, setPlaying, exporting, setExporting, pct, setPct,
    exportDone, setExportDone, exportError, setExportError, exportPath, setExportPath, retryMedia,
  } = useEditorData(folder, rev, quality);
  // Mirrors `doc` (reassigned every render) so queued closures read it fresh at execution time - see Editor.md.
  const docRef = useRef(doc);
  docRef.current = doc;

  const dur = vidDurMs > 0 ? vidDurMs : (doc?.trim.out_ms ?? 0);
  const proj = folder.split(/[\\/]/).pop() ?? "project";
  const { record, undo, redo, canUndo, canRedo } = useEditHistory(folder, docRef, setDoc, () => setRev((r) => r + 1), enqueue, onSwap);
  const director = useDirector(); // fake-pointer choreography for onRun below - see director/useDirector.ts

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

  const applyOp = (op: EditOp): Promise<EditDoc | null> => enqueue(async () => {
    if (docRef.current) record(docRef.current); // snapshot the pre-edit doc for undo, read fresh at execution time
    try {
      const d = await applyEditOp(folder, op); setDoc(d);
      if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
      return d;
    } catch { return null; }
  });
  const { moveMode, requestMoveMode, moveOffDialog } = useMoveModeGuard(doc, applyOp);
  // Only a Region-target zoom has a stored aim point, so `aimPoint` doubles as "is aiming even
  // possible right now" - a cursor-following or unselected zoom renders no reticle and leaves the
  // canvas click on its normal add-a-zoom meaning.
  const aimPoint = zoomTargetPoint(doc?.zooms.find((z) => z.id === sel)?.target);
  const aimMode = aimOn && !moveMode && !!aimPoint;
  const onMoveMode = (want: boolean) => { if (want) setAimOn(false); requestMoveMode(want); };
  const aimAt = (x: number, y: number) => { if (sel) void applyOp({ op: "update_zoom", id: sel, target: { fixed: { x, y } } }); };
  const { onTrimIn, onTrimOut, onResetTrim } = useTrimActions(doc, timeMs, dur, applyOp);
  const { saveDocSettings, onAutoModel } = useDocSettings(folder, doc, setDoc, record, setRev);
  const { addZoom, addSpotlight, addCameraMove, zoomAt } = useTimelineActions(applyOp, timeMs, setSel, setPlaying);

  useEditorKeymap({ sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight, onOverlay: () => setShowShortcuts((s) => !s) });

  // Agentic AI director: `director.run` (director/useDirector.ts) fetches the plan and
  // choreographs the fake pointer AROUND the exact apply+narrate sequence the F2 fix (commit
  // d08c71e) requires - see that hook's own comment. `running` is still set synchronously - before
  // anything is enqueued - so the re-entrancy guard AND the Transport wand's `disabled` take effect
  // immediately; the whole pass still runs as ONE `enqueue(...)` call (see Editor.md) so an
  // undo/redo issued mid-run is queued behind it, not interleaved with a reveal step's `setDoc`.
  const onRun = () => {
    if (!doc || running) return;
    setRunning(true); setAiError(null); setAiLog([]);
    void enqueue(() => director.run(folder, docRef, record, dur, setDoc, setRev, setAiLog, setAiError, setPlaying, setTimeMs)
      .finally(() => setRunning(false)));
  };

  if (!doc) return <div className="editor"><div className="e-stage-empty" style={{ margin: "auto" }}>Loading edit...</div></div>;

  const trimRange = resolveTrim(doc.trim, dur);
  const trimmed = trimRange.inMs > 0 || trimRange.outMs < dur;
  // The living brand mark's state (Task 39) - gated by the doc's own `ui.animated_brand` feel
  // knob (a per-recording settings snapshot, same as `doc.settings.cursor.pack` elsewhere) before
  // TcursorMark ever sees anything but "idle"; exporting wins over directing if somehow both were
  // true (can't actually happen - the AI director doesn't run mid-export).
  const brandState: MarkState = !doc.settings.ui.animated_brand ? "idle" : exporting ? "exporting" : running ? "directing" : "idle";

  return (
    <div className="editor">
      <ResizeEdges />
      <TopBar proj={proj} exporting={exporting} pct={pct} onOpenExport={() => setShowExportDialog(true)} onOpenSettings={() => setShowSettings(true)} onClose={onClose}
        onUndo={() => void undo()} onRedo={() => void redo()} canUndo={canUndo} canRedo={canRedo} brandState={brandState} />
      <div className="e-body">
        <Rail tab={tab} onTab={(t) => { setSel(null); setAimOn(false); setTab(t); }} />
        <EditorPanels doc={doc} sel={sel} tab={tab} dur={dur} setSel={setSel} setTab={setTab}
          timeMs={timeMs} running={running} aiError={aiError} aiLog={aiLog} aiProgress={director.progress} onRun={onRun} onAutoModel={onAutoModel} applyOp={applyOp} saveDocSettings={saveDocSettings}
          moveMode={moveMode} requestMoveMode={onMoveMode} camDraftRef={camDraftRef} osCursorInVideo={osCursor}
          aimMode={aimMode} onAimMode={setAimOn}
          addZoom={addZoom} addSpotlight={addSpotlight} addCameraMove={addCameraMove} />
        {/* Relative-positioned wrapper so the undo/redo Toast can sit bottom-center of the stage
            without Stage.tsx (already at the file line limit) needing to own toast state itself. */}
        <div className="e-stagetoast">
          <Stage src={srcUrl} webcamSrc={fileSrc(`${folder}\\webcam.webm`)} track={track} layout={layout} layoutPresets={layoutPresets} layoutSegs={doc.layout} cameraMoves={doc.camera_moves} zooms={doc.zooms} zoomSettings={doc.settings.zoom} clicks={clicks} bgUrl={bgUrl} cursorSprites={cursorSpr} cursorKinds={cursorKnd} osCursorInVideo={osCursor} cursor={doc.settings.cursor} effects={doc.effects} clickfx={doc.settings.clickfx} audioSrc={audioUrl} muted={muted} volume={volume / 100} timeMs={timeMs} playing={playing} moveMode={moveMode} aimPoint={aimPoint} aimMode={aimMode} camDraftRef={camDraftRef} onTime={onTime} onDuration={setVidDurMs} onZoomAt={zoomAt} onAimAt={aimAt} onRetryMedia={retryMedia} />
          <Toast msg={toast} onDone={dismissToast} />
        </div>
      </div>
      <DirectorOverlay running={running} planning={director.planning} model={doc.settings.ai_model || undefined} pointerRef={director.pointerRef} progress={director.progress} onCancel={director.requestCancel} />
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
        aiRunning={running}
        exporting={exporting}
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
      <Timeline doc={doc} timeMs={timeMs} dur={dur} playing={playing} onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }} sel={sel} onSel={setSel} onApply={applyOp} thumbs={thumbs} waves={waves} wavesReady={wavesReady} />
      {moveOffDialog}
      <ExportDialog open={showExportDialog} exporting={exporting} pct={pct} done={exportDone} error={exportError} exportPath={exportPath}
        onClose={() => setShowExportDialog(false)} onReset={() => { setExportDone(false); setExportError(null); setExportPath(""); }}
        onExport={(settings) => { setExportDone(false); setExportError(null); setExportPath(""); setExporting(true); setPct(0); void exportProject(folder, settings); }} />
      <ShortcutsOverlay open={showShortcuts} onClose={() => setShowShortcuts(false)} />
      <EditorSettingsDialog open={showSettings} settings={doc.settings} onClose={() => setShowSettings(false)}
        onSaveSettings={saveDocSettings} onOpenShortcuts={() => { setShowSettings(false); setShowShortcuts(true); }} />
    </div>
  );
}
