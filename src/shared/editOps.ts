import type { CamZoomAction } from "../hud/settings/settings";
import type {
  Aspect,
  CamMoveShape,
  Caption,
  EffectKind,
  PanelPose,
  TextAnchor,
  TextAnim,
  TextKind,
  TextSize,
  ZoomTarget,
} from "./edit";

export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | {
      op: "update_zoom";
      id: string;
      start_ms?: number;
      end_ms?: number;
      scale?: number;
      target?: ZoomTarget;
      easing?: string;
      easing_out?: string;
      zoom_in_ms?: number;
      zoom_out_ms?: number;
      layer?: number;
      smart_typing?: boolean;
    }
  | { op: "remove_zoom"; id: string }
  | { op: "clear_zooms" }
  | { op: "set_zoom_cam_action"; id: string; action: CamZoomAction | null }
  | { op: "set_trim"; in_ms: number; out_ms: number }
  | { op: "set_aspect"; aspect: Aspect }
  | { op: "add_cut"; start_ms: number; end_ms: number }
  | { op: "add_cuts"; spans: [number, number][] }
  | { op: "update_cut"; id: string; start_ms?: number; end_ms?: number }
  | { op: "remove_cut"; id: string }
  | { op: "set_speed"; start_ms: number; end_ms: number; factor: number }
  | { op: "update_speed"; id: string; start_ms?: number; end_ms?: number; factor?: number }
  | { op: "remove_speed"; id: string }
  | {
      op: "add_layout_seg";
      at_ms: number;
      dur_ms: number;
      layout: string;
      transition_out_ms?: number;
      easing_out?: string;
    }
  | {
      op: "update_layout_seg";
      id: string;
      start_ms?: number;
      end_ms?: number;
      layout?: string;
      transition_ms?: number;
      easing?: string;
      transition_out_ms?: number;
      easing_out?: string;
    }
  | { op: "remove_layout_seg"; id: string }
  | { op: "set_arrangement"; id: string; screen?: PanelPose | null; cam?: PanelPose | null }
  | { op: "clear_arrangement"; id: string }
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | {
      op: "update_effect";
      id: string;
      start_ms?: number;
      end_ms?: number;
      fade_in_ms?: number;
      fade_out_ms?: number;
      mode?: string;
      dim?: number;
      radius?: number;
      feather?: number;
      layer?: number;
      rect?: [number, number, number, number];
      strength?: number;
      roundness?: number;
    }
  | { op: "remove_effect"; id: string }
  | {
      op: "add_camera_move";
      t_ms: number;
      x: number;
      y: number;
      size: number;
      shape?: CamMoveShape;
      roundness?: number;
    }
  | {
      op: "update_camera_move";
      id: string;
      t_ms?: number;
      x?: number;
      y?: number;
      size?: number;
      easing?: string;
      shape?: CamMoveShape;
      roundness?: number;
    }
  | { op: "remove_camera_move"; id: string }
  | { op: "add_text"; at_ms: number; dur_ms: number; kind: TextKind }
  | {
      op: "update_text";
      id: string;
      start_ms?: number;
      end_ms?: number;
      text?: string;
      sub?: string | null;
      kind?: TextKind;
      style?: string;
      pos?: TextAnchor;
      offset?: [number, number];
      size?: TextSize;
      anim_in?: TextAnim;
      anim_out?: TextAnim;
      in_ms?: number;
      out_ms?: number;
      easing?: string;
    }
  | { op: "remove_text"; id: string }
  | { op: "split_at"; at_ms: number }
  | { op: "move_clip"; id: string; to_index: number }
  | { op: "update_clip"; id: string; src_in_ms?: number; src_out_ms?: number; transition_in_ms?: number }
  | { op: "remove_clip"; id: string }
  | { op: "apply_motion_default" }
  | { op: "update_caption"; id: string; start_ms?: number; end_ms?: number; text?: string }
  | { op: "remove_caption"; id: string }
  | { op: "merge_captions"; id: string }
  | { op: "split_caption"; id: string; at_ms: number }
  | { op: "set_captions"; captions: Caption[] }
  | { op: "clear_captions" };
