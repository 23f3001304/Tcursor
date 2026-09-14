// Audio-level math for the recording meter: RMS -> dBFS -> px, and the critically damped follower
// the meter's amplitude rides. Pure, DOM-free, frame-rate independent - every number here is named
// rather than inlined so the meter and its tests read the same constants.

/** Peak-to-peak height of a wave at silence and at 0 dBFS, in px (benchmark (d)). The voice wave
 *  is mirrored about its midline, so its half-amplitude is exactly half of each of these. */
export const AMP_MIN = 2;
export const AMP_MAX = 24;
/** Below this dBFS the take counts as silent. */
export const IDLE_DB = -50;
/** The dB window the wave spends its height on: flat at or under `FLOOR_DB`, full at or over
 *  `CEIL_DB`, linear in dB between. A mic at ordinary gain puts speech between roughly -35 and
 *  -18 dBFS RMS, and 0 dBFS is a clipped take, so the old full-scale window (-50..0) left normal
 *  speech at half height and gave the top half of the meter to a signal nobody records on purpose
 *  (owner, 2026-09-14: "the wave doesn't react much to voice"). Thirty dB centred on -31. */
export const FLOOR_DB = -46;
export const CEIL_DB = -16;

/** dBFS for a 0..1 RMS. Digital silence has no dB, so it floors at a value well under `IDLE_DB`
 *  rather than returning -Infinity (which would poison every arithmetic downstream). */
export function dbFromRms(rms: number): number {
  if (!(rms > 0)) return -120;
  return 20 * Math.log10(Math.min(1, rms));
}

/** The wave's level as a 0..1 fraction of its height for a 0..1 RMS: 0 at or under `FLOOR_DB`,
 *  1 at or over `CEIL_DB`, linear in dB between (so a halving of loudness is a constant drop,
 *  which is what makes the meter readable rather than spiky). */
export function levelFromRms(rms: number): number {
  return Math.max(0, Math.min(1, (dbFromRms(rms) - FLOOR_DB) / (CEIL_DB - FLOOR_DB)));
}

/** Peak-to-peak wave height in px for a 0..1 RMS: `AMP_MIN` at the floor, `max` (`AMP_MAX` by
 *  default; a taller slot passes its own) at the ceiling, `levelFromRms` between. */
export function heightFromRms(rms: number, max: number = AMP_MAX): number {
  return AMP_MIN + levelFromRms(rms) * (max - AMP_MIN);
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
