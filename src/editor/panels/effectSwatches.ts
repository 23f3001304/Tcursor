import type { SwatchItem } from "../controls/Controls";

export type NamedColor = [[number, number, number], string];

export const RIPPLE_COLORS: NamedColor[] = [
  [[255, 255, 255], "White"],
  [[239, 68, 68], "Red"],
  [[59, 130, 246], "Blue"],
  [[34, 197, 94], "Green"],
  [[245, 158, 11], "Orange"],
];

export const SPOTLIGHT_TINTS: NamedColor[] = [
  [[130, 90, 255], "Violet"],
  [[59, 130, 246], "Blue"],
  [[34, 197, 94], "Green"],
  [[239, 68, 68], "Red"],
  [[250, 204, 21], "Yellow"],
];

export const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

export const swatchItems = (colors: NamedColor[]): SwatchItem<[number, number, number]>[] =>
  colors.map(([c, name]) => ({ key: rgb(c), css: rgb(c), value: c, ariaLabel: name }));
