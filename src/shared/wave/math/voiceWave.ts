import { AMP_MAX, AMP_MIN, IDLE_DB, damp, dbFromRms, levelFromRms } from "./level";
import { TAU, sineY, type SineSpec } from "./sine";

export interface Layer {
  phase: number;
  freq: number;
  amp: number;
  drift: number;
  alpha: number;
}

export const LAYERS: readonly Layer[] = [
  { phase: 0.0, freq: 0.86, amp: 1.0, drift: 1.0, alpha: 0.58 },
  { phase: 2.24, freq: 1.13, amp: 0.78, drift: -0.72, alpha: 0.5 },
  { phase: 4.08, freq: 1.37, amp: 0.63, drift: 1.31, alpha: 0.44 },
  { phase: 5.6, freq: 1.58, amp: 0.51, drift: -0.55, alpha: 0.4 },
];

export const LAMBDA_PX = 58;

export const DRIFT_S = 2.4;

export const TAPER = 0.62;

export const AMP_FLOOR_PX = AMP_MIN / 2;
export const AMP_CEIL_PX = AMP_MAX / 2;

export function ceilFor(h: number): number {
  return Math.max(AMP_CEIL_PX, h / 2 - 3);
}

export const AMP_ATTACK_S = 0.02;
export const AMP_RELEASE_S = 0.065;

export const MAX_DT_S = 0.25;

export const IDLE_AFTER_S = 1.2;

export const BREATH_PERIOD_S = 2;
export const IDLE_SWELL_PX = 0.6;

export const BREATH_MAX = 1.06;

export const SAMPLE_PX = 2;

export function taperWindow(t: number, taper: number = TAPER): number {
  const r = Math.max(1e-6, Math.min(1, taper));
  const u = Math.max(0, Math.min(1, t));
  const edge = Math.min(u, 1 - u);
  if (edge >= r / 2) return 1;
  return 0.5 * (1 - Math.cos((TAU * edge) / r));
}

export interface VoiceState {
  t: number;
  amp: number;
  ampV: number;
  quietS: number;
  reduced: boolean;
}

export function initialVoiceState(reduced: boolean): VoiceState {
  return { t: 0, amp: AMP_FLOOR_PX, ampV: 0, quietS: 0, reduced };
}

export function idleAmp(t: number): number {
  return AMP_FLOOR_PX + (IDLE_SWELL_PX / 2) * (1 - Math.cos((TAU * t) / BREATH_PERIOD_S));
}

export function voiceFrame(
  s: VoiceState,
  rmsMic: number,
  rmsSys: number,
  dt: number,
  ceilPx: number = AMP_CEIL_PX,
): VoiceState {
  const d = Math.max(0, Math.min(MAX_DT_S, dt));
  const loudest = Math.max(rmsMic, rmsSys);
  const quietS = dbFromRms(loudest) < IDLE_DB ? s.quietS + d : 0;
  const t = s.reduced ? 0 : s.t + d;
  const target =
    quietS >= IDLE_AFTER_S ? idleAmp(t) : AMP_FLOOR_PX + levelFromRms(loudest) * (ceilPx - AMP_FLOOR_PX);
  if (s.reduced) return { ...s, t: 0, quietS, amp: target, ampV: 0 };
  const a = damp(s.amp, s.ampV, target, target > s.amp ? AMP_ATTACK_S : AMP_RELEASE_S, d);
  return { ...s, t, quietS, amp: a.value, ampV: a.vel };
}

export function layerSpec(s: VoiceState, layer: Layer, w: number, h: number): SineSpec {
  return {
    w,
    mid: h / 2,
    amp: s.amp * layer.amp,
    lambda: LAMBDA_PX / layer.freq,
    phase: layer.phase + (TAU * s.t * layer.drift) / DRIFT_S,
    step: SAMPLE_PX,
    envelope: taperWindow,
  };
}

export function lensPath(spec: SineSpec): string {
  const step = spec.step ?? SAMPLE_PX;
  const n = Math.max(1, Math.ceil(spec.w / step));
  const top: string[] = [];
  const bottom: string[] = [];
  for (let i = 0; i <= n; i++) {
    const x = (i / n) * spec.w;
    const y = sineY(spec, x);
    const xs = x.toFixed(2);
    top.push(`${i === 0 ? "M" : "L"} ${xs} ${y.toFixed(2)}`);
    bottom.push(`L ${xs} ${(2 * spec.mid - y).toFixed(2)}`);
  }
  bottom.reverse();
  return `${top.join(" ")} ${bottom.join(" ")} Z`;
}

export function layerPath(s: VoiceState, layer: Layer, w: number, h: number): string {
  return lensPath(layerSpec(s, layer, w, h));
}
