import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTimeMap } from "./hooks/useTimeMap";
import { useSilences } from "./hooks/useSilences";
import { applyEditOp, fileSrc, DEFAULT_PROXY_HEIGHT } from "../lib/ipc";
import type { MarkState } from "../lib/brandWave";
import { resolveTrim, type EditDoc, type EditOp } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import { TopBar } from "./shell/TopBar";
import { EditorDialogs } from "./shell/EditorDialogs";
import { useUndoToast } from "./shell/Toast";
import { ResizeEdges } from "./controls/ResizeEdges";
import { TAB_IDS, type Tab } from "./shell/panelTabs";
import { readPanelTab, writePanelTab } from "./shell/panelState";
import { ClassicShell } from "./shell/ClassicShell";
import type { ShellProps } from "./shell/slotProps";
import type { Range } from "./timeline/useRangeSelect";
import { zoomTargetPoint } from "./stage/zoomTargetMapper";
import { useEditorData } from "./hooks/useEditorData";
import { hasWebcamSignal } from "./hooks/editorData";
import { useExportState } from "./hooks/useExportState";
import { useEditorCallbacks } from "./hooks/useEditorCallbacks";
import { useEditorKeymap } from "./hooks/useEditorKeymap";
import { useMoveModeGuard } from "./hooks/useMoveModeGuard";
import { useArrangeMode } from "./hooks/useArrangeMode";
import { useTrimActions } from "./hooks/useTrimActions";
import { useEditHistory } from "./hooks/useEditHistory";
import { useDocSettings } from "./hooks/useDocSettings";
import { createQueue } from "./hooks/opQueue";
import { useDirector } from "./director/useDirector";
import { useTimelineActions } from "./hooks/useTimelineActions";
import "./editor.css";

/** The post-record editor. The preview plays the recording natively (Stage) and applies
 *  the exact camera curve (camera_track) as a transform - smooth 60fps. Edits go through
 *  the Edit API; `rev` bumps so the camera curve refetches and reflects them. */
export function Editor({ folder, onClose }: { folder: string; onClose: () => void }) {
  const [timeMs, setTimeMs] = useState(0);
  const [rev, setRev] = useState(0);
  // Which panel the rail is showing, or `null` for "collapsed". Seeded from (and written back to)
  // localStorage, so an editor reopens the way the user left it - collapsed included.
  const [tab, setTab] = useState<Tab | null>(() => readPanelTab(TAB_IDS));
  useEffect(() => { writePanelTab(tab); }, [tab]);
  const [sel, setSel] = useState<string | null>(null);
  // The ruler's Shift+drag selection, in clip ms - UI state like `sel`, shared because the timeline
  // draws it and the transport's Cut/Speed act on it (useRangeSelect.ts).
  const [range, setRange] = useState<Range | null>(null);
  // On-stage aim mode for a Region zoom. Exclusive with camera Move mode (both claim the same
  // pointer): entering Move exits aiming, and aiming stays suppressed while Move is on.
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
  // The UNSAVED Move-mode webcam pose: dragging the PiP updates it live (preview only); the Camera
  // panel's Update/Add button saves it as a keyframe. Shared so Stage (drag) and CameraPanel (save) see it.
  const camDraftRef = useRef<CamPose | null>(null);
  // Fed to useEditHistory's onSwap below (undo/redo pill); `push` is the same pill generalized to
  // also carry the export-warning text (Task 11).
  const { msg: toast, onSwap, push: pushToast, dismiss: dismissToast } = useUndoToast();
  // Serializes every EditDoc mutation (applyOp/onRun/undo/redo) through one promise chain (see
  // opQueue.ts + Editor.md) so none of them can race a still-in-flight one.
  const enqueue = useRef(createQueue()).current;

  const {
    doc, setDoc, track, layout, layoutPresets, clicks, bg, cursorSpr, cursorKnd, cursorLyr, osCursor,
    thumbs, waves, wavesReady, audioUrl, srcUrl, playing, setPlaying, retryMedia,
  } = useEditorData(folder, rev, quality);
  const exportState = useExportState(pushToast);
  const { exporting } = exportState;
  // Mirrors `doc`/`timeMs` (reassigned every render) so queued closures/tick-stable callbacks read
  // them fresh at execution time instead of closing over a stale value (Editor.md, "render hygiene").
  const docRef = useRef(doc);
  docRef.current = doc;
  const timeMsRef = useRef(timeMs);
  timeMsRef.current = timeMs;

  const dur = vidDurMs > 0 ? vidDurMs : (doc?.trim.out_ms ?? 0);
  const { map, outDoc } = useTimeMap(doc, dur); // cuts and speed spans, as one clock map and a remapped doc for the stage
  const proj = folder.split(/[\\/]/).pop() ?? "project";
  const { record, unrecord, undo, redo, canUndo, canRedo } = useEditHistory(folder, docRef, setDoc, () => setRev((r) => r + 1), enqueue, onSwap);
  const director = useDirector(); // fake-pointer choreography for onRun below - see director/useDirector.ts

  const applyOp = useCallback((op: EditOp): Promise<EditDoc | null> => enqueue(async () => {
    // `token.snapshot` is non-null only if record() actually pushed a NEW undo entry (vs
    // coalesced) - unrecord(token) below is an identity-checked no-op otherwise/always-safe. See useEditHistory.md.
    const token = docRef.current ? record(docRef.current) : null;
    try {
      const d = await applyEditOp(folder, op);
      setDoc(d); docRef.current = d; // read by a queued undo/redo BEFORE React's own re-render lands (M1)
      if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
      return d;
    } catch {
      if (token) unrecord(token); // a failed apply changed nothing - no phantom undo step either (L1)
      return null;
    }
  }), [enqueue, docRef, record, unrecord, folder, setDoc, setRev]);
  const { moveMode, requestMoveMode, moveOffDialog, moveOffOpen } = useMoveModeGuard(doc, applyOp);
  // Only a Region-target zoom has a stored aim point, so `aimPoint` doubles as "is aiming even
  // possible right now". Memoized on the found zoom (stable across ticks) rather than recomputed
  // every render: `zoomTargetPoint` returns a fresh `[x, y]` array every call, which would
  // otherwise defeat `Stage`'s memo (`aimPoint` is one of its props) on every single tick.
  const selZoom = doc?.zooms.find((z) => z.id === sel);
  const aimPoint = useMemo(() => zoomTargetPoint(selZoom?.target), [selZoom]);
  // Trim range, kept in a ref alongside `docRef`/`timeMsRef` above - `useEditorCallbacks`'s
  // `onTime`/`onPlayToggle` read it at call time so their identity doesn't depend on the trim
  // (which `resolveTrim` recomputes as a fresh object every render regardless of real changes).
  const trimRange = resolveTrim(doc?.trim ?? { in_ms: 0, out_ms: 0 }, dur);
  const trimRangeRef = useRef(trimRange);
  trimRangeRef.current = trimRange;

  const { onTime, aimAt, onMoveMode, onSeek, onPlayToggle, onAspect, cycleQuality, onMuteToggle } = useEditorCallbacks({
    applyOp, sel, dur, timeMsRef, trimRangeRef, setTimeMs, setPlaying, setAimOn, setQuality, setMuted, requestMoveMode,
  });
  // Stage arrange mode (T34) - the third exclusive stage mode, gated into `aimMode` for the same
  // reason Move is. Built after `onSeek`, which it seeks into the segment with. useArrangeMode.md.
  const { arrangeSeg, arrangeOn, onArrange, onSel } = useArrangeMode(doc, sel, { setSel, timeMsRef, onSeek });
  const aimMode = aimOn && !moveMode && !arrangeOn && !!aimPoint;
  const { onTrimIn, onTrimOut, onResetTrim } = useTrimActions(doc, timeMsRef, dur, applyOp);
  const onDetectSilences = useSilences(folder, docRef, applyOp, pushToast);
  const { saveDocSettings, onAutoModel } = useDocSettings(folder, docRef, setDoc, record, setRev, enqueue);
  const { addZoom, addSpotlight, addCameraMove, zoomAt } = useTimelineActions(applyOp, timeMsRef, docRef, setSel, setPlaying);

  // Inert behind any modal (M4), incl. the AI director's own scrim (`running`). `shortcutsOpen`
  // lets "?" still toggle ShortcutsOverlay closed while it's the one open (keymap.ts).
  const modalOpen = showExportDialog || showShortcuts || showSettings || moveOffOpen || running;
  useEditorKeymap({ sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight, onOverlay: () => setShowShortcuts((s) => !s), modalOpen, shortcutsOpen: showShortcuts, escOwned: arrangeOn });

  // Agentic AI director: `director.run` fetches the plan and choreographs the fake pointer
  // (see that hook's own comment). `running` is set synchronously so the wand's `disabled` takes
  // effect immediately; the whole pass runs as ONE `enqueue(...)` so a mid-run undo/redo queues
  // behind it. `exporting` guard (L3) belt-and-braces Transport's wand + AiPanel's own `disabled`.
  const onRun = useCallback(() => {
    if (!doc || running || exporting) return;
    setRunning(true); setAiError(null); setAiLog([]);
    void enqueue(() => director.run(folder, docRef, record, dur, setDoc, setRev, setAiLog, setAiError, setPlaying, setTimeMs)
      .finally(() => setRunning(false)));
  }, [doc, running, exporting, enqueue, director.run, folder, docRef, record, dur, setDoc, setRev]);

  if (!doc) return <div className="editor"><div className="e-stage-empty" style={{ margin: "auto" }}>Loading edit...</div></div>;

  const trimmed = trimRange.inMs > 0 || trimRange.outMs < dur;
  // The living brand mark's state (Task 39) - gated by the doc's own `ui.animated_brand` feel
  // knob (a per-recording settings snapshot, same as `doc.settings.cursor.pack` elsewhere) before
  // TcursorMark ever sees anything but "idle"; exporting wins over directing if somehow both were
  // true - `onRun`'s own `exporting` guard above is what actually enforces that, not just this order.
  const brandState: MarkState = !doc.settings.ui.animated_brand ? "idle" : exporting ? "exporting" : running ? "directing" : "idle";
  // Everything the four editor types need, named after these locals so the hand-off stays a
  // shorthand literal. `ClassicShell` composes them (rail, one panel slot, stage, transport,
  // timeline); this file only assembles the props.
  const shell: ShellProps = {
    folder, doc, outDoc: outDoc ?? doc, map, range, setRange, sel, setSel, onSel, tab, setTab, dur, timeMs, timeMsRef, playing, muted, volume, setVolume, modalOpen,
    quality, cycleQuality, onMuteToggle, exporting, running, trimmed, aimMode, aimPoint, setAimOn, moveMode,
    onMoveMode, arrangeSeg, arrangeOn, onArrange, camDraftRef, srcUrl, audioUrl, bg, track, layout,
    layoutPresets, clicks, cursorSpr, cursorKnd, cursorLyr, osCursor, thumbs, waves, wavesReady,
    webcamSrc: fileSrc(`${folder}\\webcam.webm`), hasWebcam: hasWebcamSignal(layout), aspectLocked: exporting || dur <= 0,
    onTime, onSeek, onPlayToggle, onAspect, onDuration: setVidDurMs, zoomAt, aimAt, applyOp, saveDocSettings,
    retryMedia, addZoom, addSpotlight, addCameraMove, onRun, onAutoModel, aiError, aiLog, toast, dismissToast,
    aiProgress: director.progress, aiPlanning: director.planning, pointerRef: director.pointerRef,
    onCancelRun: director.requestCancel, onTrimIn, onTrimOut, onResetTrim, onDetectSilences,
  };

  return (
    <div className="editor">
      <ResizeEdges />
      <TopBar proj={proj} exporting={exporting} pct={exportState.pct} onOpenExport={() => setShowExportDialog(true)} onOpenSettings={() => setShowSettings(true)} onClose={onClose}
        onUndo={() => void undo()} onRedo={() => void redo()} canUndo={canUndo} canRedo={canRedo} brandState={brandState} />
      <ClassicShell {...shell} />
      {moveOffDialog}
      <EditorDialogs folder={folder} settings={doc.settings} exportState={exportState} onSaveSettings={saveDocSettings}
        showExport={showExportDialog} onCloseExport={() => setShowExportDialog(false)}
        showShortcuts={showShortcuts} onCloseShortcuts={() => setShowShortcuts(false)} showSettings={showSettings}
        onCloseSettings={() => setShowSettings(false)} onOpenShortcuts={() => { setShowSettings(false); setShowShortcuts(true); }} />
    </div>
  );
}
