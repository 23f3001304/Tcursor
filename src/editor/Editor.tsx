import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { applyProjectUi } from "./shell/settings/applyUi";
import { DEFAULT_PROXY_HEIGHT } from "../shared/ipc";
import type { MarkState } from "../shared/brand/brandWave";
import { resolveTrim } from "../shared/edit";
import type { CamPose } from "./stage/camera/cameraMoves";
import { TopBar } from "./shell/TopBar";
import { EditorDialogs } from "./shell/dialogs/EditorDialogs";
import { ResizeEdges } from "./controls/surfaces/ResizeEdges";
import { TAB_IDS, type Tab } from "./shell/PanelTabs";
import { readPanelTab, writePanelTab } from "./shell/panelState";
import { ClassicShell } from "./shell/ClassicShell";
import { useDensityVars } from "./shell/useDensity";
import type { Range } from "./timeline/useRangeSelect";
import { zoomTargetPoint } from "./stage/camera/zoomTargetMapper";
import { buildShellProps } from "./model/shellProps";
import { useEditorSession } from "./hooks/doc/useEditorSession";
import { useEditorCallbacks } from "./hooks/input/useEditorCallbacks";
import { useEditorKeymap } from "./hooks/input/useEditorKeymap";
import { useMoveModeGuard } from "./hooks/input/useMoveModeGuard";
import { useArrangeMode } from "./hooks/input/useArrangeMode";
import { useEditorModals } from "./hooks/input/useEditorModals";
import { useTrimActions } from "./hooks/doc/useTrimActions";
import { useSilences } from "./hooks/doc/useSilences";
import { useAiRun } from "./director/useAiRun";
import { useTimelineActions } from "./hooks/doc/useTimelineActions";
import "./editor.css";

export function Editor({ folder, onClose }: { folder: string; onClose: () => void }) {
  const dvars = useDensityVars();
  const [timeMs, setTimeMs] = useState(0);
  const [tab, setTab] = useState<Tab | null>(() => readPanelTab(TAB_IDS));
  useEffect(() => {
    writePanelTab(tab);
  }, [tab]);
  const [sel, setSel] = useState<string | null>(null);
  const [range, setRange] = useState<Range | null>(null);
  const [aimOn, setAimOn] = useState(false);
  const [vidDurMs, setVidDurMs] = useState(0);
  const [quality, setQuality] = useState(DEFAULT_PROXY_HEIGHT);
  const [muted, setMuted] = useState(false);
  const [volume, setVolume] = useState(100);
  const camDraftRef = useRef<CamPose | null>(null);

  const session = useEditorSession(folder, quality, vidDurMs);
  const { doc, docRef, dur, applyOp, setPlaying, exportState, pushToast } = session;
  const { exporting } = exportState;
  const ui = doc?.settings.ui;
  useEffect(() => {
    if (ui) applyProjectUi(ui);
  }, [ui]);
  const timeMsRef = useRef(timeMs);
  timeMsRef.current = timeMs;
  const proj = folder.split(/[\\/]/).pop() ?? "project";

  const { moveMode, requestMoveMode, moveOffDialog, moveOffOpen } = useMoveModeGuard(
    doc,
    applyOp,
    camDraftRef,
  );
  const selZoom = doc?.zooms.find((z) => z.id === sel);
  const aimPoint = useMemo(() => zoomTargetPoint(selZoom?.target), [selZoom]);
  const trimRange = resolveTrim(doc?.trim ?? { in_ms: 0, out_ms: 0 }, dur);
  const trimRangeRef = useRef(trimRange);
  trimRangeRef.current = trimRange;

  const cb = useEditorCallbacks({
    applyOp,
    sel,
    dur,
    timeMsRef,
    trimRangeRef,
    setTimeMs,
    setPlaying,
    setAimOn,
    setQuality,
    setMuted,
    requestMoveMode,
  });
  const arrange = useArrangeMode(doc, sel, { setSel, timeMsRef, onSeek: cb.onSeek });
  const aimMode = aimOn && !moveMode && !arrange.arrangeOn && !!aimPoint;
  const trim = useTrimActions(doc, timeMsRef, dur, applyOp);
  const onDetectSilences = useSilences(folder, docRef, applyOp, pushToast);
  const timeline = useTimelineActions(applyOp, timeMsRef, docRef, setSel, setPlaying);
  const ai = useAiRun({
    folder,
    docRef,
    dur,
    enqueue: session.enqueue,
    record: session.record,
    setDoc: session.setDoc,
    bumpRev: session.bumpRev,
    onSeek: cb.onSeek,
    setPlaying,
  });

  const modals = useEditorModals(moveOffOpen);
  const modalOpen = modals.modalOpen;
  useEditorKeymap({
    sel,
    doc,
    timeMs,
    setSel,
    setPlaying,
    applyOp,
    addZoom: timeline.addZoom,
    addSpotlight: timeline.addSpotlight,
    onOverlay: modals.toggleShortcuts,
    modalOpen,
    shortcutsOpen: modals.shortcutsOpen,
    escOwned: arrange.arrangeOn,
  });

  const onRun = useCallback(() => {
    if (!exporting) void ai.run();
  }, [exporting, ai.run]);

  if (!doc)
    return (
      <div className="editor" style={dvars}>
        <div className="e-stage-empty" style={{ margin: "auto" }}>
          Loading edit...
        </div>
      </div>
    );

  const brandState: MarkState = !doc.settings.ui.animated_brand
    ? "idle"
    : exporting
      ? "exporting"
      : ai.running
        ? "directing"
        : "idle";
  const shell = buildShellProps({
    folder,
    doc,
    session,
    cb,
    arrange,
    timeline,
    trim,
    ai,
    view: {
      sel,
      setSel,
      tab,
      setTab,
      range,
      setRange,
      timeMs,
      timeMsRef,
      muted,
      volume,
      setVolume,
      quality,
      modalOpen,
      trimmed: trimRange.inMs > 0 || trimRange.outMs < dur,
      aimMode,
      aimPoint,
      setAimOn,
      moveMode,
      camDraftRef,
      onDuration: setVidDurMs,
      onDetectSilences,
      onRun,
    },
  });

  return (
    <div className="editor" style={dvars}>
      <ResizeEdges />
      <TopBar
        proj={proj}
        exporting={exporting}
        pct={exportState.pct}
        onOpenExport={modals.openExport}
        onOpenSettings={modals.openSettings}
        onClose={onClose}
        onUndo={() => void session.undo()}
        onRedo={() => void session.redo()}
        canUndo={session.canUndo}
        canRedo={session.canRedo}
        brandState={brandState}
      />
      <ClassicShell {...shell} />
      {moveOffDialog}
      <EditorDialogs
        folder={folder}
        settings={doc.settings}
        exportState={exportState}
        onSaveSettings={session.saveDocSettings}
        onApplyMotion={() => void applyOp({ op: "apply_motion_default" })}
        {...modals.dialog}
      />
    </div>
  );
}
