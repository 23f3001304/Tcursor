import type { BackgroundSettings } from "../../../hud/settings/settings";
import type { SwatchItem } from "../../controls/Controls";

export const DEFAULT_BG: BackgroundSettings = {
  kind: "mesh",
  solid: [24, 24, 30],
  mesh: "",
  gradient_mid: null,
  gradient_from: [36, 41, 56],
  gradient_to: [88, 64, 120],
  gradient_angle_deg: 135,
  blur: 0,
  asset: null,
  dim: 0,
};

export interface NamedColor {
  rgb: [number, number, number];
  name: string;
}
export const COLOR_PRESETS: NamedColor[] = [
  { rgb: [9, 9, 11], name: "Dark Zinc" },
  { rgb: [24, 24, 27], name: "Zinc 900" },
  { rgb: [45, 26, 74], name: "Deep Violet" },
  { rgb: [20, 61, 44], name: "Emerald Forest" },
  { rgb: [30, 41, 59], name: "Slate 800" },
  { rgb: [88, 28, 135], name: "Grape Purple" },
  { rgb: [124, 45, 18], name: "Warm Rust" },
  { rgb: [49, 46, 129], name: "Indigo Midnight" },
  { rgb: [15, 118, 110], name: "Teal 700" },
  { rgb: [190, 24, 93], name: "Magenta Rose" },
  { rgb: [55, 65, 81], name: "Steel Gray" },
  { rgb: [17, 24, 39], name: "Charcoal" },
];

export const ACCENTS: NamedColor[] = [
  { rgb: [239, 68, 68], name: "Red" },
  { rgb: [124, 108, 240], name: "Purple" },
  { rgb: [59, 130, 246], name: "Blue" },
  { rgb: [34, 197, 94], name: "Green" },
  { rgb: [245, 158, 11], name: "Orange" },
];

export const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

export const COLOR_ITEMS: SwatchItem<[number, number, number]>[] = COLOR_PRESETS.map((p) => ({
  key: rgb(p.rgb),
  css: rgb(p.rgb),
  value: p.rgb,
  ariaLabel: p.name,
}));
