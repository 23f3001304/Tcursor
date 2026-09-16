import type { RefObject } from "react";
import type {
  CamSample,
  ClickSample,
  PreviewLayout,
  CursorKindSample,
  LayoutPresets,
} from "../../../shared/ipc";
import type {
  CaptionStyle,
  ClickFxSettings,
  CursorSettings,
  GradeSettings,
  ZoomSettings,
} from "../../../hud/settings/settings";
import type { Caption, CameraMove, EffectRegion, LayoutSeg, TextItem, Zoom } from "../../../shared/edit";
import type { CamPose } from "../../stage/camera/cameraMoves";
import type { StageBgState } from "../../stage/canvas/stageBg";
import type { SpotlightSimState } from "../../stage/fx/spotlightPreview";
import type { TimeMap } from "../../../shared/math/remap";
import type { ExactFrame } from "./useExactFrame";
import type { CursorSpritesState } from "./useCursorSprites";

export interface CompositeLoopRefs {
  screenRef: RefObject<HTMLVideoElement | null>;
  webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  playRef: RefObject<boolean>;
  timeRef: RefObject<number>;
  onTimeRef: RefObject<(ms: number) => void>;
  trackRef: RefObject<CamSample[]>;
  layoutRef: RefObject<PreviewLayout | null>;
  layoutPresetsRef: RefObject<LayoutPresets | null>;
  layoutSegsRef: RefObject<LayoutSeg[]>;
  cameraMovesRef: RefObject<CameraMove[]>;
  zoomsRef: RefObject<Zoom[]>;
  zoomSettingsRef: RefObject<ZoomSettings>;
  dragPoseRef: RefObject<CamPose | null>;
  arrangingRef: RefObject<boolean>;
  clicksRef: RefObject<ClickSample[]>;
  effectsRef: RefObject<EffectRegion[]>;
  clickfxRef: RefObject<ClickFxSettings>;
  gradeRef: RefObject<GradeSettings>;
  captionsRef: RefObject<Caption[]>;
  textsRef: RefObject<TextItem[]>;
  capStyleRef: RefObject<CaptionStyle>;
  accentRef: RefObject<[number, number, number]>;
  kindsRef: RefObject<CursorKindSample[]>;
  cursorRef: RefObject<CursorSettings>;
  spritesRef: RefObject<CursorSpritesState>;
  trailRef: RefObject<[number, number][]>;
  dirtyRef: RefObject<boolean>;
  bgRef: RefObject<StageBgState | null>;
  spotSimRef: RefObject<SpotlightSimState>;
  mapRef: RefObject<TimeMap>;
  exactRef: RefObject<ExactFrame | null>;
  editGenRef: RefObject<number>;
}
