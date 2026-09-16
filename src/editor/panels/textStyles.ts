import type { TextKind } from "../../shared/edit";

export interface TextStyle {
  fill: "white" | "accent";
  shadow: boolean;
  plate: boolean;
  plateRgb: [number, number, number];
  plateAlpha: number;
  rule: boolean;
}

const CLEAN: TextStyle = {
  fill: "white",
  shadow: true,
  plate: false,
  plateRgb: [0, 0, 0],
  plateAlpha: 0,
  rule: false,
};

export const TEXT_STYLES: Record<string, TextStyle> = {
  clean: CLEAN,
  plate: { ...CLEAN, shadow: false, plate: true, plateAlpha: 0.62 },
  accent: { ...CLEAN, fill: "accent" },
  bar: { ...CLEAN, shadow: false, rule: true },
};

export const styleOf = (name: string): TextStyle => TEXT_STYLES[name] ?? CLEAN;

export const TEXT_STYLE_OPTIONS: { value: string; label: string }[] = [
  { value: "clean", label: "Clean" },
  { value: "plate", label: "Plate" },
  { value: "accent", label: "Accent" },
  { value: "bar", label: "Bar" },
];

export const TEXT_KIND_OPTIONS: { value: TextKind; label: string }[] = [
  { value: "title", label: "Title" },
  { value: "lower_third", label: "Lower third" },
  { value: "stat", label: "Stat" },
  { value: "callout", label: "Callout" },
];

export const TEXT_PILLS: { kind: TextKind; name: string; hint: string }[] = [
  { kind: "title", name: "Title", hint: "A line across the middle of the frame" },
  { kind: "lower_third", name: "Lower Third", hint: "A name and a role in the corner" },
  { kind: "stat", name: "Big Stat", hint: "One number and its caption" },
  { kind: "callout", name: "Callout", hint: "A short line that types itself in" },
];
