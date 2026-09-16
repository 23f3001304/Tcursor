import { LUMA, TEMP_GAIN, TINT_GAIN, vignetteK, type GradeParams } from "./gradeParams";

const clamp01 = (x: number) => Math.min(1, Math.max(0, x));

export function gradePixel(
  c: [number, number, number],
  p: GradeParams,
  u: number,
  v: number,
): [number, number, number] {
  const e = Math.pow(2, p.exposure);
  const o: [number, number, number] = [
    c[0] * e * (1 + TEMP_GAIN * p.temp),
    c[1] * e * (1 + TINT_GAIN * p.tint),
    c[2] * e * (1 - TEMP_GAIN * p.temp),
  ];
  for (let i = 0; i < 3; i++) {
    const x = clamp01(p.lift[i] + (p.gain[i] - p.lift[i]) * o[i]);
    o[i] = Math.pow(x, 1 / Math.max(p.gamma[i], 0.001));
  }
  for (let i = 0; i < 3; i++) o[i] = (o[i] - 0.5) * p.contrast + 0.5;
  const l = LUMA[0] * o[0] + LUMA[1] * o[1] + LUMA[2] * o[2];
  for (let i = 0; i < 3; i++) o[i] = l + (o[i] - l) * p.saturation;
  const k = vignetteK(u, v);
  const m = 1 - p.vignette * k * k;
  return [clamp01(o[0] * m), clamp01(o[1] * m), clamp01(o[2] * m)];
}
