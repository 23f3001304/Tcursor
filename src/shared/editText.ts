export type TextKind = "title" | "lower_third" | "stat" | "callout";
export type TextAnchor =
  | "top_left"
  | "top_center"
  | "top_right"
  | "mid_left"
  | "mid_center"
  | "mid_right"
  | "bottom_left"
  | "bottom_center"
  | "bottom_right";
export type TextSize = "xs" | "s" | "m" | "l" | "xl";
export type TextAnim = "fade" | "slide" | "pop" | "typewriter";

export const TEXT_SIZE_FRACS: Record<TextSize, number> = {
  xs: 0.03,
  s: 0.042,
  m: 0.058,
  l: 0.082,
  xl: 0.115,
};

export interface TextItem {
  id: string;
  start_ms: number;
  end_ms: number;
  kind: TextKind;
  text: string;
  sub?: string | null;
  style: string;
  pos: TextAnchor;
  offset: [number, number];
  size: TextSize;
  anim_in: TextAnim;
  anim_out: TextAnim;
  in_ms: number;
  out_ms: number;
  easing: string;
}
