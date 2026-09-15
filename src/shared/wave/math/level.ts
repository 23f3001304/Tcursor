export const AMP_MIN = 2;
export const AMP_MAX = 24;

export const IDLE_DB = -50;

export const FLOOR_DB = -46;
export const CEIL_DB = -16;

export function dbFromRms(rms: number): number {
  if (!(rms > 0)) return -120;
  return 20 * Math.log10(Math.min(1, rms));
}

export function levelFromRms(rms: number): number {
  return Math.max(0, Math.min(1, (dbFromRms(rms) - FLOOR_DB) / (CEIL_DB - FLOOR_DB)));
}

export function heightFromRms(rms: number, max: number = AMP_MAX): number {
  return AMP_MIN + levelFromRms(rms) * (max - AMP_MIN);
}

export interface Damped {
  value: number;
  vel: number;
}

export function damp(value: number, vel: number, target: number, tau: number, dt: number): Damped {
  if (!(dt > 0)) return { value, vel };
  const w = 1 / tau;
  const decay = Math.exp(-w * dt);
  const e0 = value - target;
  const b = vel + w * e0;
  return { value: target + (e0 + b * dt) * decay, vel: (vel - w * b * dt) * decay };
}
