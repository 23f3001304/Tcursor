import type { TextAnchor, TextItem } from "../../shared/edit";
import type { GraphInput } from "../motion/graphModel";

export const ANCHOR_GRID: TextAnchor[] = [
  "top_left",
  "top_center",
  "top_right",
  "mid_left",
  "mid_center",
  "mid_right",
  "bottom_left",
  "bottom_center",
  "bottom_right",
];

export const ANIM_OPTIONS = [
  { value: "fade", label: "Fade" },
  { value: "slide", label: "Slide" },
  { value: "pop", label: "Pop" },
  { value: "typewriter", label: "Typewriter" },
] as const;

export const SIZE_OPTIONS = [
  { value: "xs", label: "XS" },
  { value: "s", label: "S" },
  { value: "m", label: "M" },
  { value: "l", label: "L" },
  { value: "xl", label: "XL" },
] as const;

export const slideIsPointless = (pos: TextAnchor) => pos === "mid_center";

export function textGraphInput(item: TextItem): GraphInput {
  return {
    lane: "text",
    startMs: item.start_ms,
    endMs: item.end_ms,
    peak: 1,
    rampIn: { easing: item.easing, durMs: item.in_ms },
    rampOut: { easing: item.easing, durMs: item.out_ms },
  };
}
