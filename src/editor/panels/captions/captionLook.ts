import type { NamedColor } from "../effectSwatches";
import type { CaptionAnim, CaptionSize, CaptionStyle } from "../../../hud/settings/settings";

export const SIZE_RUNG_PCT: Record<CaptionSize, number> = { s: 3.0, m: 3.8, l: 4.8 };

export const LINE_PCT_MIN = 2.0;
export const LINE_PCT_MAX = 6.0;
export const LINE_PCT_STEP = 0.1;

export const ANIM_MS_MAX = 600;
export const ANIM_MS_STEP = 20;

export const captionLinePct = (s: CaptionStyle): number =>
  s.font_pct > 0 ? s.font_pct : SIZE_RUNG_PCT[s.size];

export function captionRung(s: CaptionStyle): CaptionSize | "" {
  const pct = captionLinePct(s);
  const keys = Object.keys(SIZE_RUNG_PCT) as CaptionSize[];
  return keys.find((k) => Math.abs(SIZE_RUNG_PCT[k] - pct) < 0.001) ?? "";
}

export const linePctText = (v: number) => `${v.toFixed(1)}% of height`;

export const ANIMATIONS: { value: CaptionAnim; label: string; title: string }[] = [
  { value: "none", label: "None", title: "The line appears and goes with no animation." },
  { value: "fade", label: "Fade", title: "The line fades in, and fades out again." },
  { value: "rise", label: "Rise", title: "The line fades in while drifting up into place." },
  { value: "pop", label: "Pop", title: "The line fades in while growing to full size." },
  {
    value: "words",
    label: "Word by word",
    title: "Each word appears as you say it. Fades instead when the line has no word timings.",
  },
];

export const TEXT_COLORS: NamedColor[] = [
  [[255, 255, 255], "White"],
  [[245, 238, 224], "Warm white"],
  [[250, 204, 21], "Yellow"],
  [[103, 232, 249], "Cyan"],
];

export const HIGHLIGHT_COLORS: NamedColor[] = [
  [[250, 204, 21], "Yellow"],
  [[74, 222, 128], "Green"],
  [[96, 165, 250], "Blue"],
  [[248, 113, 113], "Red"],
];

export const PILL_COLORS: NamedColor[] = [
  [[0, 0, 0], "Black"],
  [[16, 24, 44], "Deep navy"],
  [[255, 255, 255], "White"],
];
