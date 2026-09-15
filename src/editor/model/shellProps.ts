import type { Dispatch, RefObject, SetStateAction } from "react";
import type { EditDoc } from "../../shared/edit";
import type { ShellProps } from "../shell/slotProps";
import type { Tab } from "../shell/PanelTabs";
import type { Range } from "../timeline/useRangeSelect";
import type { useEditorSession } from "../hooks/doc/useEditorSession";
import type { useEditorCallbacks } from "../hooks/input/useEditorCallbacks";
import type { useArrangeMode } from "../hooks/input/useArrangeMode";
import type { useTimelineActions } from "../hooks/doc/useTimelineActions";
import type { useTrimActions } from "../hooks/doc/useTrimActions";
import type { useAiRun } from "../director/useAiRun";
import { fileSrc } from "../../shared/ipc";
import { hasWebcamSignal } from "./editorData";

export interface ShellView {
  sel: string | null;
  setSel: Dispatch<SetStateAction<string | null>>;
  tab: Tab | null;
  setTab: Dispatch<SetStateAction<Tab | null>>;
  range: Range | null;
  setRange: Dispatch<SetStateAction<Range | null>>;
  timeMs: number;
  timeMsRef: RefObject<number>;
  muted: boolean;
  volume: number;
  setVolume: Dispatch<SetStateAction<number>>;
  quality: number;
  modalOpen: boolean;
  trimmed: boolean;
  aimMode: boolean;
  aimPoint: [number, number] | null;
  setAimOn: Dispatch<SetStateAction<boolean>>;
  moveMode: boolean;
  camDraftRef: ShellProps["camDraftRef"];
  onDuration: (ms: number) => void;
  onDetectSilences: () => void;
  onRun: () => void;
}

export function buildShellProps(a: {
  folder: string;
  doc: EditDoc;
  session: ReturnType<typeof useEditorSession>;
  cb: ReturnType<typeof useEditorCallbacks>;
  arrange: ReturnType<typeof useArrangeMode>;
  timeline: ReturnType<typeof useTimelineActions>;
  trim: ReturnType<typeof useTrimActions>;
  ai: ReturnType<typeof useAiRun>;
  view: ShellView;
}): ShellProps {
  const { folder, doc, session: s, cb, arrange, timeline, trim, ai, view } = a;
  return {
    folder,
    doc,
    outDoc: s.outDoc ?? doc,
    map: s.map,
    dur: s.dur,
    playing: s.playing,
    exporting: s.exportState.exporting,
    running: ai.running,
    srcUrl: s.srcUrl,
    audioUrl: s.audioUrl,
    bg: s.bg,
    track: s.track,
    layout: s.layout,
    layoutPresets: s.layoutPresets,
    clicks: s.clicks,
    cursorSpr: s.cursorSpr,
    cursorKnd: s.cursorKnd,
    cursorLyr: s.cursorLyr,
    osCursor: s.osCursor,
    thumbs: s.thumbs,
    waves: s.waves,
    wavesReady: s.wavesReady,
    webcamSrc: fileSrc(`${folder}\\webcam.webm`),
    hasWebcam: hasWebcamSignal(s.layout),
    aspectLocked: s.exportState.exporting || s.dur <= 0,
    applyOp: s.applyOp,
    saveDocSettings: s.saveDocSettings,
    reloadDoc: s.reloadDoc,
    retryMedia: s.retryMedia,
    onAutoModel: s.onAutoModel,
    toast: s.toast,
    dismissToast: s.dismissToast,
    ...view,
    ...cb,
    ...arrange,
    ...timeline,
    ...trim,
    aiError: ai.error,
    aiRun: ai.aiRun,
    aiSkipped: ai.skipped,
    aiApplying: ai.applying,
    aiPreviewId: ai.previewId,
    onToggleItem: ai.toggleItem,
    onPreviewItem: ai.preview,
    onApplyRun: ai.apply,
    onDiscardRun: ai.discard,
    stageOutline: ai.outline,
    aiProgress: ai.progress,
    aiPlanning: ai.planning,
    pointerRef: ai.pointerRef,
    onCancelRun: ai.requestCancel,
  };
}
