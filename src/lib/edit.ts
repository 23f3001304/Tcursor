import type { Settings } from "../hud/settings/settings";

export type ZoomTarget = "cursor" | { fixed: { x: number; y: number } };

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
}

export interface Cut { start_ms: number; end_ms: number }
export interface Speed { id: string; start_ms: number; end_ms: number; factor: number }
export interface LayoutSeg { id: string; start_ms: number; end_ms: number; layout: string; transition_ms: number; easing: string }
export interface Trim { in_ms: number; out_ms: number }
export type EffectKind = "spotlight";
export interface EffectRegion { id: string; kind: EffectKind; start_ms: number; end_ms: number; fade_in_ms: number; fade_out_ms: number; mode?: string; dim?: number; radius?: number; feather?: number; layer: number }
export interface CameraMove { id: string; t_ms: number; x: number; y: number; size: number; easing: string }

export interface EditDoc {
  version: number;
  trim: Trim;
  cuts: Cut[];
  zooms: Zoom[];
  speed: Speed[];
  layout: LayoutSeg[];
  effects: EffectRegion[];
  camera_moves: CameraMove[];
  settings: Settings;
}

export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | { op: "update_zoom"; id: string; start_ms?: number; end_ms?: number; scale?: number; target?: ZoomTarget; easing?: string; zoom_in_ms?: number; zoom_out_ms?: number; layer?: number }
  | { op: "remove_zoom"; id: string }
  | { op: "set_trim"; in_ms: number; out_ms: number }
  | { op: "add_cut"; start_ms: number; end_ms: number }
  | { op: "set_speed"; start_ms: number; end_ms: number; factor: number }
  | { op: "add_layout_seg"; at_ms: number; dur_ms: number; layout: string }
  | { op: "update_layout_seg"; id: string; start_ms?: number; end_ms?: number; layout?: string; transition_ms?: number; easing?: string }
  | { op: "remove_layout_seg"; id: string }
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | { op: "update_effect"; id: string; start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number; layer?: number }
  | { op: "remove_effect"; id: string }
  | { op: "add_camera_move"; t_ms: number; x: number; y: number; size: number }
  | { op: "update_camera_move"; id: string; t_ms?: number; x?: number; y?: number; size?: number; easing?: string }
  | { op: "remove_camera_move"; id: string };
