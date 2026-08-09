import type { Settings } from "../hud/settings/settings";

export type ZoomTarget = "cursor" | { fixed: { x: number; y: number } };

// Defined next to ZoomSettings (mirroring Rust, where it lives in settings::model) and
// re-exported here so edit-doc consumers can import it alongside Zoom.
export type { CamZoomAction } from "../hud/settings/settings";
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
  /** Per-zoom webcam override; absent = inherit the global default. */
  cam_action?: CamZoomAction | null;
}

export interface Cut { start_ms: number; end_ms: number }
export interface Speed { id: string; start_ms: number; end_ms: number; factor: number }
/** `transition_out_ms` is the cross-fade OUT of this layout, completing AT `end_ms` (symmetric
 *  with the entry, which starts at `start_ms`). `0` is a hard cut - what every doc written before
 *  exit transitions existed deserializes to. Rust always serializes both fields, so they are
 *  required here even though they are serde-defaulted on the wire. */
export interface LayoutSeg { id: string; start_ms: number; end_ms: number; layout: string; transition_ms: number; easing: string; transition_out_ms: number; easing_out: string }
export interface Trim { in_ms: number; out_ms: number }

/** Output frame aspect ratio - mirrors Rust `export::types::Aspect`. `"source"` (the default)
 *  matches today's behavior (the frame adapts to the recording's own dimensions); the 4 fixed
 *  presets pin a base resolution at that ratio. Never crops: the screen aspect-fits inside the
 *  (possibly resized) frame and the background fills the rest. */
export type Aspect = "source" | "wide_16x9" | "vertical_9x16" | "square_1x1" | "classic_4x3";

/** Effective trim range against the clip's real duration - mirrors Rust `Trim::resolve` exactly,
 *  so the preview (playhead clamp, timeline dimming) always agrees with what export will cut.
 *  `trim.out_ms === 0` (unset) reads as "no trim yet": the whole clip. */
export function resolveTrim(trim: Trim, durMs: number): { inMs: number; outMs: number } {
  const outMs = trim.out_ms === 0 ? durMs : Math.min(trim.out_ms, durMs);
  const inMs = Math.min(trim.in_ms, outMs);
  return { inMs, outMs };
}

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
  aspect: Aspect;
  settings: Settings;
}

export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | { op: "update_zoom"; id: string; start_ms?: number; end_ms?: number; scale?: number; target?: ZoomTarget; easing?: string; zoom_in_ms?: number; zoom_out_ms?: number; layer?: number }
  | { op: "remove_zoom"; id: string }
  | { op: "clear_zooms" }
  | { op: "set_zoom_cam_action"; id: string; action: CamZoomAction | null }
  | { op: "set_trim"; in_ms: number; out_ms: number }
  | { op: "set_aspect"; aspect: Aspect }
  | { op: "add_cut"; start_ms: number; end_ms: number }
  | { op: "set_speed"; start_ms: number; end_ms: number; factor: number }
  | { op: "add_layout_seg"; at_ms: number; dur_ms: number; layout: string; transition_out_ms?: number; easing_out?: string }
  | { op: "update_layout_seg"; id: string; start_ms?: number; end_ms?: number; layout?: string; transition_ms?: number; easing?: string; transition_out_ms?: number; easing_out?: string }
  | { op: "remove_layout_seg"; id: string }
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | { op: "update_effect"; id: string; start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number; layer?: number }
  | { op: "remove_effect"; id: string }
  | { op: "add_camera_move"; t_ms: number; x: number; y: number; size: number }
  | { op: "update_camera_move"; id: string; t_ms?: number; x?: number; y?: number; size?: number; easing?: string }
  | { op: "remove_camera_move"; id: string };
