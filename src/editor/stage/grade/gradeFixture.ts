import type { GradePreset } from "../../../hud/settings/settings";
import { paramsOf, seedOf, type GradeParams } from "./gradeParams";
import { gradePixel } from "./gradeCpu";

export const COLOURS: [number, number, number][] = [
  [0, 0, 0],
  [255, 255, 255],
  [128, 128, 128],
  [255, 0, 0],
  [0, 255, 0],
  [0, 0, 255],
  [255, 255, 0],
  [0, 255, 255],
  [255, 0, 255],
  [224, 172, 148],
  [18, 24, 48],
  [239, 68, 68],
];

export const PRESETS: GradePreset[] = [
  "none",
  "cinematic",
  "noir",
  "vintage",
  "frost",
  "golden",
  "midnight",
  "vivid",
  "dreamy",
];

export const IDENTITY: GradeParams = {
  exposure: 0,
  contrast: 1,
  vignette: 0,
  saturation: 1,
  temp: 0,
  tint: 0,
  lift: [0, 0, 0],
  gamma: [1, 1, 1],
  gain: [1, 1, 1],
};

export function graded(p: GradePreset, c: [number, number, number], u: number, v: number) {
  const [exposure, contrast, vignette] = seedOf(p);
  const params = paramsOf({ preset: p, exposure, contrast, vignette }) ?? IDENTITY;
  const f = gradePixel([c[0] / 255, c[1] / 255, c[2] / 255], params, u, v);
  return [Math.round(f[0] * 255), Math.round(f[1] * 255), Math.round(f[2] * 255)];
}
