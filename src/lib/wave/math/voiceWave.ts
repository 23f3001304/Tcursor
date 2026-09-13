// The recording meter's voice wave, as pure math: the layer table, the window that tapers the
// wave into the flat line at both ends, the level's ballistics, and the closed mirrored outline
// each layer draws. `VoiceWave.tsx` owns only a rAF loop and four `setAttribute` calls;
// everything that could be wrong - the log mapping, the attack/release, the idle breath, the
// reduced-motion freeze, the taper - lives here and is tested.

import { AMP_MAX, AMP_MIN, IDLE_DB, damp, dbFromRms, heightFromRms } from "./level";
import { TAU, sineY, type SineSpec } from "./sine";

/** One drawn layer of the wave.
 *
 *  `freq` scales the base wavelength, `amp` the live amplitude, `alpha` is the layer's fill
 *  opacity. `drift` scales the base phase speed and its SIGN is the direction that layer travels:
 *  giving neighbours opposite signs is what makes the layers sweep THROUGH each other rather than
 *  slide along in formation, which is the whole reason there is more than one of them. */
export interface Layer { phase: number; freq: number; amp: number; drift: number; alpha: number }

/** The four layers, back to front - and that order is also the paint order, so the widest and
 *  most opaque one sits underneath and the thinner, more transparent ones ride over it. Nothing
 *  here paints a second colour to fake depth: every layer is filled from the same blue-to-violet
 *  ramp, and the places where two of them cross are darker purely because two translucent fills
 *  composited there.
 *
 *  The numbers are deliberately not ratios of small integers. At 0.86 / 1.13 / 1.37 / 1.58 the
 *  four wavelengths share no common period inside the meter's width, so the crossings never land
 *  in the same place twice and the shape does not visibly loop. */
export const LAYERS: readonly Layer[] = [
  { phase: 0.00, freq: 0.86, amp: 1.00, drift: 1.00, alpha: 0.58 },
  { phase: 2.24, freq: 1.13, amp: 0.78, drift: -0.72, alpha: 0.50 },
  { phase: 4.08, freq: 1.37, amp: 0.63, drift: 1.31, alpha: 0.44 },
  { phase: 5.60, freq: 1.58, amp: 0.51, drift: -0.55, alpha: 0.40 },
];

/** Base spatial period, px - the wavelength of a layer whose `freq` is 1. */
export const LAMBDA_PX = 58;
/** Seconds for a layer whose `drift` is 1 to advance one full turn of phase. */
export const DRIFT_S = 2.4;
/** Fraction of the width spent on the two cosine tapers (half at each end). The middle is left
 *  flat, so the wave stays legible across most of a 104px meter instead of bulging in the centre
 *  the way a plain Hann window would. */
export const TAPER = 0.62;
/** Half-amplitudes, px: the flat-line floor at silence and the ceiling at 0 dBFS. Derived from
 *  `level.ts`'s peak-to-peak pair rather than restated, since the lens is mirrored about the
 *  midline and its half-height IS that peak-to-peak height halved. */
export const AMP_FLOOR_PX = AMP_MIN / 2;
export const AMP_CEIL_PX = AMP_MAX / 2;
/** Lag constants of the amplitude follower, seconds. Critically damped (`damp`) reaches ~95% of a
 *  step in 4.74 tau, so 0.02 is a ~95ms rise and 0.065 a ~300ms fall: a syllable lands at once,
 *  and the wave settles back slowly enough to read as a decay rather than a flicker. */
export const AMP_ATTACK_S = 0.02;
export const AMP_RELEASE_S = 0.065;
/** A frame longer than this (a suspended window, a stalled main thread) is treated as this long,
 *  so the wave resumes from where it was instead of teleporting. */
export const MAX_DT_S = 0.25;
/** Continuous silence before the wave starts breathing instead of lying dead flat. */
export const IDLE_AFTER_S = 1.2;
/** The idle breath: its cycle, and how far above the floor it swells, px. Kept inside the
 *  reference's 1-2px window, so a silent meter reads as "listening", never as a second signal. */
export const BREATH_PERIOD_S = 2;
export const IDLE_SWELL_PX = 0.6;
/** Peak scale of the brand dot's own breathe - the mark's idle tell, used by `QuietWaves.tsx`.
 *  It lives beside the wave's breath because the two share `BREATH_PERIOD_S`: one cycle, so the
 *  dot and the wave cannot be tuned apart. */
export const BREATH_MAX = 1.06;
/** Sampling step of the outline, px. */
export const SAMPLE_PX = 2;

/** The amplitude envelope across the meter, for a normalised x (0..1).
 *
 *  A cosine-tapered (Tukey) window: 0 at both edges, rising over the first `taper / 2` of the
 *  width, flat at 1 across the middle, falling again at the far edge. Zero at the ends is the
 *  point - it is what lets the wave dissolve into the centre line instead of being chopped off at
 *  the meter's border, and it is why the end dots always sit on an unbroken line. */
export function taperWindow(t: number, taper: number = TAPER): number {
  const r = Math.max(1e-6, Math.min(1, taper));
  const u = Math.max(0, Math.min(1, t));
  const edge = Math.min(u, 1 - u);
  if (edge >= r / 2) return 1;
  return 0.5 * (1 - Math.cos((TAU * edge) / r));
}

export interface VoiceState {
  /** Phase clock, seconds. Frozen at 0 under reduced motion. */
  t: number;
  /** Half-amplitude of the widest layer, px, and its follower velocity. */
  amp: number; ampV: number;
  /** Seconds of continuous silence seen. */
  quietS: number;
  reduced: boolean;
}

/** A wave that has just mounted: flat, silent, and not yet breathing (so a take that starts loud
 *  never shows the idle swell first). */
export function initialVoiceState(reduced: boolean): VoiceState {
  return { t: 0, amp: AMP_FLOOR_PX, ampV: 0, quietS: 0, reduced };
}

/** The silent wave's half-amplitude at clock `t`: the floor, plus a slow cosine swell of
 *  `IDLE_SWELL_PX`. Never reaches the amplitude any real signal would produce. */
export function idleAmp(t: number): number {
  return AMP_FLOOR_PX + (IDLE_SWELL_PX / 2) * (1 - Math.cos((TAU * t) / BREATH_PERIOD_S));
}

/** Advance the wave by `dt` seconds given the latest mic and system RMS (both 0..1). Pure: the
 *  caller keeps the returned state and hands it back next frame.
 *
 *  One amplitude drives all four layers, taken from whichever source is louder - the meter answers
 *  "how loud is what this take is recording", and a second amplitude would only ask the viewer to
 *  tell two overlapping translucent shapes apart at 30px tall, which nobody can do. */
export function voiceFrame(s: VoiceState, rmsMic: number, rmsSys: number, dt: number): VoiceState {
  const d = Math.max(0, Math.min(MAX_DT_S, dt));
  const loudest = Math.max(rmsMic, rmsSys);
  const quietS = dbFromRms(loudest) < IDLE_DB ? s.quietS + d : 0;
  const t = s.reduced ? 0 : s.t + d;
  const target = quietS >= IDLE_AFTER_S ? idleAmp(t) : heightFromRms(loudest) / 2;
  if (s.reduced) return { ...s, t: 0, quietS, amp: target, ampV: 0 };
  const a = damp(s.amp, s.ampV, target, target > s.amp ? AMP_ATTACK_S : AMP_RELEASE_S, d);
  return { ...s, t, quietS, amp: a.value, ampV: a.vel };
}

/** The sine one layer is riding this frame, in the meter's own px space. */
export function layerSpec(s: VoiceState, layer: Layer, w: number, h: number): SineSpec {
  return {
    w, mid: h / 2, amp: s.amp * layer.amp, lambda: LAMBDA_PX / layer.freq,
    phase: layer.phase + (TAU * s.t * layer.drift) / DRIFT_S,
    step: SAMPLE_PX, envelope: taperWindow,
  };
}

/** A closed outline for one layer: the sine across the width, then its mirror image back again,
 *  joined into a single fillable shape.
 *
 *  Mirroring rather than filling down to the midline is what produces the lens shapes. The two
 *  halves meet wherever the sine crosses zero, so a layer is a chain of pinched lobes above and
 *  below the line - the reference's shape - and at the two ends the taper closes it to a point on
 *  the line itself. */
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

/** `lensPath` of `layerSpec` - the one call a frame makes per layer. */
export function layerPath(s: VoiceState, layer: Layer, w: number, h: number): string {
  return lensPath(layerSpec(s, layer, w, h));
}
