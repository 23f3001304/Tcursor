import type { CamShape, CamCorner } from "./settings";

export type ModeKey = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
export const MODES: [ModeKey, string][] = [
  ["screen", "Screen"], ["camera", "Camera"], ["presenter", "Presenter"],
  ["screen_only", "S-only"], ["camera_only", "C-only"],
];

export type Knob = "pad" | "screen_size" | "screen_radius" | "cam_size" | "cam_radius" | "cam_margin_x" | "cam_margin_y";
type Spec = { label: string; min: number; max: number; step: number };
export const SLIDERS: Record<Knob, Spec> = {
  pad:           { label: "Padding",       min: 0,    max: 0.08, step: 0.002 },
  screen_size:   { label: "Screen size",   min: 0.6,  max: 1.0,  step: 0.01 },
  screen_radius: { label: "Corner radius", min: 0,    max: 0.05, step: 0.002 },
  cam_size:      { label: "Webcam size",   min: 0.08, max: 1.0,  step: 0.01 },
  cam_radius:    { label: "Corner roundness", min: 0,    max: 0.5,  step: 0.01 },
  cam_margin_x:  { label: "Margin X",      min: 0,    max: 0.1,  step: 0.002 },
  cam_margin_y:  { label: "Margin Y",      min: 0,    max: 0.1,  step: 0.002 },
};

// Which sliders each mode shows (per the design matrix). Shape/corner are separate.
export const MODE_SLIDERS: Record<ModeKey, Knob[]> = {
  screen:      ["pad", "screen_size", "screen_radius", "cam_size", "cam_margin_x", "cam_margin_y"],
  screen_only: ["pad", "screen_size", "screen_radius"],
  camera:      ["pad", "screen_size", "screen_radius", "cam_size"],
  camera_only: ["pad", "cam_size"],
  presenter:   ["pad", "screen_radius"],
};
export const MODE_HAS_SHAPE: Record<ModeKey, boolean> = { screen: true, camera: true, camera_only: true, presenter: true, screen_only: false };
export const MODE_HAS_CORNER: Record<ModeKey, boolean> = { screen: true, camera: false, camera_only: false, presenter: false, screen_only: false };

export const SHAPES: [CamShape, string][] = [["circle", "Circle"], ["rounded", "Rounded"], ["rect", "Rect"]];
export const CORNERS: [CamCorner, string][] = [["bottom_left", "BL"], ["bottom_right", "BR"], ["top_left", "TL"], ["top_right", "TR"]];

export const pct = (v: number) => `${Math.round(v * 100)}%`;
