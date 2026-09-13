// Audio-level math for the recording meter: RMS -> dBFS -> px, and the critically damped follower
// the meter's amplitude rides. Pure, DOM-free, frame-rate independent - every number here is named
// rather than inlined so the meter and its tests read the same constants.

/** Peak-to-peak height of a wave at silence and at 0 dBFS, in px (benchmark (d)). The voice wave
 *  is mirrored about its midline, so its half-amplitude is exactly half of each of these. */
export const AMP_MIN = 2;
export const AMP_MAX = 24;
/** Below this dBFS the take counts as silent. */
export const IDLE_DB = -50;

/** dBFS for a 0..1 RMS. Digital silence has no dB, so it floors at a value well under `IDLE_DB`
 *  rather than returning -Infinity (which would poison every arithmetic downstream). */
export function dbFromRms(rms: number): number {
  if (!(rms > 0)) return -120;
  return 20 * Math.log10(Math.min(1, rms));
}

/** Peak-to-peak wave height in px for a 0..1 RMS: log-mapped, `AMP_MIN` at or under `IDLE_DB`,
 *  `AMP_MAX` at 0 dBFS, linear in dB between the two (so a halving of loudness is a constant
 *  drop in px, which is what makes the meter readable rather than spiky). */
export function heightFromRms(rms: number): number {
  const t = Math.max(0, Math.min(1, (dbFromRms(rms) - IDLE_DB) / -IDLE_DB));
  return AMP_MIN + t * (AMP_MAX - AMP_MIN);
}

export interface Damped { value: number; vel: number }

/** One step of a critically damped spring, solved analytically rather than integrated.
 *
 *  `tau` is the lag constant in seconds: the response reaches ~26% of a step after `tau`, ~95%
 *  after `5*tau`, and never overshoots. Because it is the closed-form solution it is
 *  unconditionally stable and frame-rate independent - two 8ms steps land exactly where one 16ms
 *  step does, so a meter on a 144Hz panel and one on a 60Hz panel read the same. */
export function damp(value: number, vel: number, target: number, tau: number, dt: number): Damped {
  if (!(dt > 0)) return { value, vel };
  const w = 1 / tau;
  const decay = Math.exp(-w * dt);
  const e0 = value - target;          // error coordinates: the spring always pulls e -> 0
  const b = vel + w * e0;             // x(t) = (e0 + b t) e^-wt  is the critically damped solution
  return { value: target + (e0 + b * dt) * decay, vel: (vel - w * b * dt) * decay };
}
