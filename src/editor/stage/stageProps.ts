import type { RefObject } from "react";
import type {
  CamSample,
  ClickSample,
  CursorPackDto,
  CursorKindSample,
  CursorLayerDto,
  PreviewLayout,
  LayoutPresets,
} from "../../shared/ipc";
import type {
  CaptionStyle,
  CursorSettings,
  ClickFxSettings,
  ZoomSettings,
} from "../../hud/settings/settings";
import type { Caption, CameraMove, EditDoc, EditOp, EffectRegion, LayoutSeg, Zoom } from "../../shared/edit";
import type { CamPose } from "./camera/cameraMoves";
import type { StageBg } from "./canvas/stageBg";
import type { TimeMap } from "../../shared/math/remap";

export interface StageProps {
  folder: string;
  src: string;
  webcamSrc: string;
  track: CamSample[];
  layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null;
  layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[];
  zooms: Zoom[];
  zoomSettings: ZoomSettings;
  clicks: ClickSample[];
  bg: StageBg;
  map: TimeMap;
  cursorSprites: CursorPackDto | null;
  cursorKinds: CursorKindSample[];
  cursorLayer: CursorLayerDto | null;
  osCursorInVideo: boolean;
  cursor: CursorSettings;
  effects: EffectRegion[];
  clickfx: ClickFxSettings;
  captions: Caption[];
  capStyle: CaptionStyle;
  accent: [number, number, number];
  audioSrc: string;
  muted: boolean;
  volume: number;
  timeMs: number;
  playing: boolean;
  moveMode: boolean;
  aimPoint: [number, number] | null;
  aimMode: boolean;
  arrangeSeg: LayoutSeg | null;
  outline: [number, number, number, number] | null;
  camDraftRef: RefObject<CamPose | null>;
  onTime: (ms: number) => void;
  onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void;
  onAimAt: (x: number, y: number) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onRetryMedia: () => void;
}
