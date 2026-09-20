import type { Dispatch, RefObject, SetStateAction } from "react";
import type { TimeMap } from "../../shared/math/remap";
import type { Range } from "../timeline/useRangeSelect";
import type { Aspect, EditDoc, EditOp, LayoutSeg, MaskKind, TextKind } from "../../shared/edit";
import type {
  CamSample,
  ClickSample,
  CursorKindSample,
  CursorLayerDto,
  CursorPackDto,
  LayoutPresets,
  PreviewLayout,
} from "../../shared/ipc";
import type { CamPose } from "../stage/camera/cameraMoves";
import type { StageBg } from "../stage/canvas/stageBg";
import type { DirectorPointerHandle } from "../director/DirectorPointer";
import type { AiRun } from "../../shared/aiRun";
import type { ToastMsg } from "./dialogs/Toast";
import type { Tab } from "./PanelTabs";

export interface SlotProps {
  folder: string;
  doc: EditDoc;
  sel: string | null;
  setSel: Dispatch<SetStateAction<string | null>>;
  onSel: (id: string | null) => void;
  tab: Tab | null;
  setTab: Dispatch<SetStateAction<Tab | null>>;
  onTab: (t: Tab) => void;
  dur: number;
  timeMs: number;
  timeMsRef: RefObject<number>;
  playing: boolean;
  muted: boolean;
  volume: number;
  setVolume: Dispatch<SetStateAction<number>>;
  quality: number;
  cycleQuality: () => void;
  onMuteToggle: () => void;
  exporting: boolean;
  running: boolean;
  trimmed: boolean;
  aimMode: boolean;
  aimPoint: [number, number] | null;
  setAimOn: Dispatch<SetStateAction<boolean>>;
  moveMode: boolean;
  onMoveMode: (want: boolean) => void;
  arrangeSeg: LayoutSeg | null;
  arrangeOn: boolean;
  onArrange: () => void;
  camDraftRef: RefObject<CamPose | null>;
  srcUrl: string;
  webcamSrc: string;
  audioUrl: string;
  bg: StageBg;
  map: TimeMap;
  outDoc: EditDoc;
  range: Range | null;
  setRange: Dispatch<SetStateAction<Range | null>>;
  track: CamSample[];
  layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null;
  clicks: ClickSample[];
  cursorSpr: CursorPackDto | null;
  cursorKnd: CursorKindSample[];
  cursorLyr: CursorLayerDto | null;
  osCursor: boolean;
  thumbs: string[];
  waves: { system: string; mic: string };
  wavesReady: boolean;
  hasWebcam: boolean;
  aspectLocked: boolean;
  onTime: (ms: number) => void;
  onSeek: (ms: number) => void;
  onPlayToggle: () => void;
  onAspect: (a: Aspect) => void;
  onDuration: (ms: number) => void;
  zoomAt: (x: number, y: number) => void;
  aimAt: (x: number, y: number) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  reloadDoc: () => void;
  retryMedia: () => void;
  addZoom: () => void;
  addSpotlight: () => void;
  addMask: (kind: MaskKind) => void;
  addCameraMove: () => void;
  addText: (kind: TextKind) => void;
  onRun: () => void;
  onAutoModel: (v: string) => void;
  aiError: string | null;
  aiProgress: { step: number; total: number } | null;
  aiPlanning: boolean;
  aiRun: AiRun | null;
  aiSkipped: ReadonlySet<string>;
  aiApplying: boolean;
  aiPreviewId: string | null;
  onToggleItem: (id: string) => void;
  onPreviewItem: (id: string) => void;
  onApplyRun: () => void;
  onDiscardRun: () => void;
  stageOutline: [number, number, number, number] | null;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  onCancelRun: () => void;
  toast: ToastMsg | null;
  dismissToast: () => void;
  onTrimIn: () => void;
  onTrimOut: () => void;
  onResetTrim: () => void;
  onDetectSilences: () => void;
  onSplit: () => void;
}

export type ShellProps = Omit<SlotProps, "onTab"> & {
  modalOpen: boolean;
};
