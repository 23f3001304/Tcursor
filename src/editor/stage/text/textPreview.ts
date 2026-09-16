import { TEXT_SIZE_FRACS } from "../../../shared/edit";
import type { TextAnchor, TextItem } from "../../../shared/edit";
import { ease } from "../../timeline/model/layoutTrack";
import { styleOf } from "../../panels/textStyles";

export const ADV = 0.52;
export const LINE_H = 1.32;
export const SUB_RATIO = 0.58;
export const PAD_X = 0.6;
export const PAD_Y = 0.34;
export const MARGIN = 0.06;
export const RULE_W = 0.006;
export const SLIDE_FRAC = 0.04;
export const POP_FROM = 0.86;

export interface LaidText {
  main: string;
  sub: string | null;
  fontPx: number;
  subPx: number;
  plate: [number, number, number, number];
  mainBaseline: [number, number];
  subBaseline: [number, number];
  rule: [number, number, number, number];
  alpha: number;
  reveal: number | null;
  scale: number;
  shift: [number, number];
  fill: [number, number, number];
  shadow: boolean;
  plateRgb: [number, number, number];
  plateAlpha: number;
  ruleRgb: [number, number, number];
  centred: boolean;
  boxW: number;
}

const anchorFrac = (a: TextAnchor): [number, number] => [
  a.endsWith("_left") ? 0 : a.endsWith("_right") ? 1 : 0.5,
  a.startsWith("top_") ? 0 : a.startsWith("bottom_") ? 1 : 0.5,
];

const slideDir = (a: TextAnchor): [number, number] =>
  a.startsWith("top_")
    ? [0, -1]
    : a.startsWith("bottom_")
      ? [0, 1]
      : a === "mid_left"
        ? [-1, 0]
        : a === "mid_right"
          ? [1, 0]
          : [0, 0];

const place = (a: number, span: number, block: number, margin: number) =>
  a === 0 ? margin : a === 1 ? span - block - margin : (span - block) / 2;

const clamp01 = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v);

export function laidTexts(
  items: TextItem[],
  accent: [number, number, number],
  w: number,
  h: number,
  tMs: number,
): LaidText[] {
  const out: LaidText[] = [];
  for (const item of items) {
    const l = layOne(item, accent, w, h, tMs);
    if (l) out.push(l);
  }
  return out;
}

export function layOne(
  item: TextItem,
  accent: [number, number, number],
  ow: number,
  oh: number,
  tMs: number,
): LaidText | null {
  if (tMs < item.start_ms || tMs >= item.end_ms) return null;
  const main = item.text.trim();
  const subTrimmed = (item.sub ?? "").trim();
  const sub = subTrimmed === "" ? null : subTrimmed;
  if (main === "" && sub === null) return null;

  const p = clamp01((tMs - item.start_ms) / Math.max(1, item.in_ms));
  const q = clamp01((item.end_ms - tMs) / Math.max(1, item.out_ms));
  const eIn = ease(item.easing, p);
  const eOut = ease(item.easing, q);
  const g = Math.min(eIn, eOut);
  const anim = eIn <= eOut ? item.anim_in : item.anim_out;

  const st = styleOf(item.style);
  const scale = anim === "pop" ? POP_FROM + (1 - POP_FROM) * g : 1;
  const fontPx = Math.max(oh * TEXT_SIZE_FRACS[item.size], 8) * scale;
  const subPx = fontPx * SUB_RATIO;
  const mainChars = [...main].length;
  const subChars = sub === null ? 0 : [...sub].length;
  const textW = Math.max(mainChars * fontPx * ADV, subChars * subPx * ADV);
  const padX = st.plate ? 2 * fontPx * PAD_X : 0;
  const ruleW = st.rule ? oh * RULE_W : 0;
  const blockW = textW + padX + ruleW * 2;
  const blockH = fontPx * LINE_H + (sub === null ? 0 : subPx * LINE_H) + (st.plate ? 2 * fontPx * PAD_Y : 0);

  const [ax, ay] = anchorFrac(item.pos);
  const margin = oh * MARGIN;
  const [dx, dy] = slideDir(item.pos);
  const slide = anim === "slide" ? (1 - g) * oh * SLIDE_FRAC : 0;
  const shift: [number, number] = [slide * dx, slide * dy];
  const bx = place(ax, ow, blockW, margin) + item.offset[0] * ow + shift[0];
  const by = place(ay, oh, blockH, margin) + item.offset[1] * oh + shift[1];

  const textX = bx + padX * 0.5 + (st.rule && ax <= 0.5 ? ruleW * 2 : 0);
  const textTop = by + (st.plate ? fontPx * PAD_Y : 0);
  return {
    main,
    sub,
    fontPx,
    subPx,
    plate: st.plate ? [bx, by, blockW, blockH] : [0, 0, 0, 0],
    mainBaseline: [textX, textTop + fontPx],
    subBaseline: [textX, textTop + fontPx * LINE_H + subPx],
    rule: st.rule ? [ax > 0.5 ? bx + blockW - ruleW : bx, by, ruleW, blockH] : [0, 0, 0, 0],
    alpha: anim === "typewriter" ? (p > 0 ? 1 : 0) * Math.min(1, q * 4) : g,
    reveal: item.anim_in === "typewriter" ? Math.floor(p * mainChars) : null,
    scale,
    shift,
    fill: st.fill === "accent" ? accent : [255, 255, 255],
    shadow: st.shadow,
    plateRgb: st.plateRgb,
    plateAlpha: st.plateAlpha,
    ruleRgb: accent,
    centred: ax === 0.5,
    boxW: textW,
  };
}
