import type { Dispatch, RefObject, SetStateAction } from "react";
import type { TimeMap } from "../../lib/remap";
import type { Range } from "../timeline/useRangeSelect";
import type { Aspect, EditDoc, EditOp, LayoutSeg } from "../../lib/edit";
import type { CamSample, ClickSample, CursorKindSample, CursorLayerDto, CursorPackDto, LayoutPresets, PreviewLayout } from "../../lib/ipc";
import type { CamPose } from "../stage/cameraMoves";
import type { StageBg } from "../stage/stageBg";
import type { DirectorPointerHandle } from "../director/DirectorPointer";
import type { ToastMsg } from "./Toast";
import type { Tab } from "./panelTabs";

/** Everything the four editor types need, in one bundle. `Editor.tsx` owns all of it and hands it
 *  to `ClassicShell` as a single object; `ClassicShell` is what spreads it back out into the props
 *  `Stage` / `Transport` / `Timeline` / `EditorPanels` / the inspectors already take. Field names
 *  match `Editor.tsx`'s own locals so the hand-off stays a shorthand object literal rather than
 *  seventy re-typed attributes.
 *
 *  Anything derivable from `doc` (aspect, zooms, layout segments, cursor/clickfx settings, the AI
 *  model name) is deliberately NOT a field - the slots read it off `doc` instead. */
export interface SlotProps {
  folder: string;
  doc: EditDoc;
  /** Selection is UI state, independent of which editor an area shows: it only ever changes what a
   *  `properties` area renders. */
  sel: string | null;
  setSel: Dispatch<SetStateAction<string | null>>;
  /** The timeline's own selection handler (`useArrangeMode`), not a bare `setSel`. */
  onSel: (id: string | null) => void;
  tab: Tab;
  setTab: Dispatch<SetStateAction<Tab>>;
  /** Opens a panel tab in the nearest `panel` area, making one if the workspace has none - built
   *  by `ClassicShell`, used by the stage toolbar's quick-open buttons. */
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
  /** The clip-to-output clock map and the doc's regions on the output clock (`useTimeMap`): the
   *  stage evaluates `outDoc`; the timeline keeps `doc`, whose pills sit on clip time. */
  map: TimeMap; outDoc: EditDoc;
  /** The ruler's Shift+drag selection, in clip ms: the timeline draws it, the transport's Cut and
   *  Speed act on it. `null` is "no range", which is what both of those fall back from. */
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
  retryMedia: () => void;
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
  onRun: () => void;
  onAutoModel: (v: string) => void;
  aiError: string | null;
  aiLog: string[];
  aiProgress: { step: number; total: number } | null;
  aiPlanning: boolean;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  onCancelRun: () => void;
  toast: ToastMsg | null;
  dismissToast: () => void;
  onTrimIn: () => void;
  onTrimOut: () => void;
  onResetTrim: () => void;
  /** Remove silences (`useSilences`): scan the audio, apply the quiet stretches as one `add_cuts`. */
  onDetectSilences: () => void;
}

/** What `Editor.tsx` passes down: the same bundle minus `onTab`, which the shell derives (it also
 *  drops the selection), plus the one flag only the shell needs. */
export type ShellProps = Omit<SlotProps, "onTab"> & {
  /** True while any dialog, overlay or the AI director's scrim owns the screen. The editors don't
   *  see it (it isn't part of `SlotProps`); the shell's own keyboard - `Ctrl+Space` to maximize -
   *  goes inert behind it, exactly like every shortcut in `keymap.ts` does. */
  modalOpen: boolean;
};
