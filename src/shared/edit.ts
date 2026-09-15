import type { Settings } from "../hud/settings/settings";

export type ZoomTarget = "cursor" | { fixed: { x: number; y: number } };

export type { CamZoomAction } from "../hud/settings/settings";
export type { EditOp } from "./editOps";
import type { CamZoomAction } from "../hud/settings/settings";

export interface Zoom {
  id: string;
  start_ms: number;
  end_ms: number;
  target: ZoomTarget;
  scale: number;
  easing: string;
  zoom_in_ms: number;
  zoom_out_ms: number;
  layer: number;
  cam_action?: CamZoomAction | null;
  smart_typing?: boolean;
  easing_out?: string | null;
}

export interface Cut {
  id: string;
  start_ms: number;
  end_ms: number;
}
export interface Speed {
  id: string;
  start_ms: number;
  end_ms: number;
  factor: number;
}

export interface PanelPose {
  cx: number;
  cy: number;
  size: number;
}

export interface Arrangement {
  screen: PanelPose | null;
  cam: PanelPose | null;
}

export interface LayoutSeg {
  id: string;
  start_ms: number;
  end_ms: number;
  layout: string;
  transition_ms: number;
  easing: string;
  transition_out_ms: number;
  easing_out: string;
  arrangement?: Arrangement | null;
}
export interface Trim {
  in_ms: number;
  out_ms: number;
}

export type Aspect = "source" | "wide_16x9" | "vertical_9x16" | "square_1x1" | "classic_4x3";

export function resolveTrim(trim: Trim, durMs: number): { inMs: number; outMs: number } {
  const outMs = trim.out_ms === 0 ? durMs : Math.min(trim.out_ms, durMs);
  const inMs = Math.min(trim.in_ms, outMs);
  return { inMs, outMs };
}

export type EffectKind = "spotlight";
export interface EffectRegion {
  id: string;
  kind: EffectKind;
  start_ms: number;
  end_ms: number;
  fade_in_ms: number;
  fade_out_ms: number;
  mode?: string;
  dim?: number;
  radius?: number;
  feather?: number;
  layer: number;
}

export interface CaptionWord {
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface Caption {
  id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  words: CaptionWord[];
}

export type CamMoveShape = "layout" | "circle" | "rounded" | "rect";
export interface CameraMove {
  id: string;
  t_ms: number;
  x: number;
  y: number;
  size: number;
  easing: string;
  shape: CamMoveShape;
  roundness: number;
}

export interface EditDoc {
  version: number;
  trim: Trim;
  cuts: Cut[];
  zooms: Zoom[];
  speed: Speed[];
  layout: LayoutSeg[];
  effects: EffectRegion[];
  camera_moves: CameraMove[];
  aspect: Aspect;
  settings: Settings;
  captions: Caption[];
  clip_ms: number;
}
