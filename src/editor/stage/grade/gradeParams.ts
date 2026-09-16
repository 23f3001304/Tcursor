import type { GradePreset, GradeSettings } from "../../../hud/settings/settings";

export const LUMA: [number, number, number] = [0.2126, 0.7152, 0.0722];
export const VIGN_IN = 0.45;
export const CORNER = 0.70710678;
export const TEMP_GAIN = 0.25;
export const TINT_GAIN = 0.2;

export interface GradeParams {
  exposure: number;
  contrast: number;
  vignette: number;
  saturation: number;
  temp: number;
  tint: number;
  lift: [number, number, number];
  gamma: [number, number, number];
  gain: [number, number, number];
}

type Row = [number, number, number, number, number, number, number[], number[], number[]];

const ROWS: Record<GradePreset, Row> = {
  none: [0.0, 1.0, 0.0, 1.0, 0.0, 0.0, [0, 0, 0], [1, 1, 1], [1, 1, 1]],
  cinematic: [0.0, 1.12, 0.28, 0.92, -0.08, 0.02, [0.012, 0.016, 0.03], [1.0, 1.0, 1.04], [1.0, 0.99, 0.96]],
  noir: [0.05, 1.3, 0.4, 0.0, 0.0, 0.0, [0.01, 0.01, 0.01], [0.96, 0.96, 0.96], [1.02, 1.02, 1.02]],
  vintage: [0.02, 0.92, 0.34, 0.78, 0.14, 0.04, [0.045, 0.035, 0.02], [1.05, 1.02, 0.98], [0.97, 0.96, 0.92]],
  frost: [0.04, 1.06, 0.16, 0.86, -0.22, -0.03, [0.01, 0.018, 0.03], [1.0, 1.02, 1.05], [0.98, 1.0, 1.03]],
  golden: [0.08, 1.05, 0.22, 1.06, 0.2, 0.05, [0.02, 0.012, 0.0], [1.02, 1.0, 0.96], [1.04, 1.0, 0.93]],
  midnight: [
    -0.12,
    1.18,
    0.42,
    0.84,
    -0.16,
    -0.06,
    [0.0, 0.004, 0.03],
    [0.96, 0.98, 1.06],
    [0.94, 0.97, 1.05],
  ],
  vivid: [0.04, 1.14, 0.1, 1.28, 0.02, 0.0, [0, 0, 0], [1, 1, 1], [1, 1, 1]],
  dreamy: [0.1, 0.88, 0.18, 1.1, 0.06, 0.03, [0.055, 0.05, 0.06], [1.06, 1.05, 1.06], [0.98, 0.98, 1.0]],
};

const triple = (v: number[]): [number, number, number] => [v[0], v[1], v[2]];

export function seedOf(p: GradePreset): [number, number, number] {
  const r = ROWS[p] ?? ROWS.none;
  return [r[0], r[1], r[2]];
}

export function isIdentityGrade(s: GradeSettings): boolean {
  return s.preset === "none" && s.exposure === 0 && s.contrast === 1 && s.vignette === 0;
}

export function paramsOf(s: GradeSettings): GradeParams | null {
  if (isIdentityGrade(s)) return null;
  const r = ROWS[s.preset] ?? ROWS.none;
  return {
    exposure: s.exposure,
    contrast: s.contrast,
    vignette: s.vignette,
    saturation: r[3],
    temp: r[4],
    tint: r[5],
    lift: triple(r[6]),
    gamma: triple(r[7]),
    gain: triple(r[8]),
  };
}

export function vignetteK(u: number, v: number): number {
  const d = Math.sqrt(u * u + v * v) / CORNER;
  return Math.min(1, Math.max(0, (d - VIGN_IN) / (1 - VIGN_IN)));
}
