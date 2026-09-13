# src/lib/wave/math/level.ts

Audio-level math for the recording meter: RMS to dBFS to px, and the critically damped follower the meter's amplitude rides. Pure, DOM-free and frame-rate independent. Every number here is named rather than inlined, so the meter and its tests read the same constants. Tests: `level.test.ts`.

The band quantiser (`BANDS` / `bandOf` / `heightOfBand`) and `CLIP_DB` went out with `meterFrame.ts`: the voice wave that replaced it has no discrete amplitude steps and no clip colour.

## AMP_MIN

```ts
export const AMP_MIN: number   // 2
```

Peak-to-peak height of a wave at silence, px. A flat-but-visible line, never a bare dot. `voiceWave.ts` halves it for `AMP_FLOOR_PX`, since the voice wave is mirrored about its midline.

## AMP_MAX

```ts
export const AMP_MAX: number   // 24
```

Peak-to-peak height at 0 dBFS, px. Halved for `AMP_CEIL_PX`, the same way.

## IDLE_DB

```ts
export const IDLE_DB: number   // -50
```

Below this the take counts as silent: the log mapping bottoms out here, and `voiceFrame` starts its idle timer.

## dbFromRms

```ts
export function dbFromRms(rms: number): number
```

dBFS for a 0..1 RMS.

### Returns

`20 * log10(rms)`, with `rms` clamped at 1. Digital silence has no dB, so a zero or negative input floors at `-120` rather than returning `-Infinity`, which would poison every arithmetic downstream.

### Behaviors

- Full scale is 0 dBFS; each halving of RMS is -6 dB.
- Zero and negative inputs both give the same finite floor, well under `IDLE_DB`.

## heightFromRms

```ts
export function heightFromRms(rms: number): number
```

Peak-to-peak wave height in px for a 0..1 RMS.

### Returns

`AMP_MIN` at or under `IDLE_DB`, `AMP_MAX` at 0 dBFS, linear in **dB** between the two - so a halving of loudness is a constant drop in px, which is what makes the meter readable rather than spiky. Clamped at both ends.

## Damped

```ts
export interface Damped { value: number; vel: number }
```

One follower's position and velocity, carried between frames by the caller.

## damp

```ts
export function damp(value: number, vel: number, target: number, tau: number, dt: number): Damped
```

One step of a critically damped spring, solved analytically rather than integrated.

### Inputs

- `value`, `vel` - last frame's state.
- `target` - where the spring is pulling.
- `tau` - the lag constant in seconds.
- `dt` - elapsed seconds. A zero or negative `dt` is a no-op.

### Behaviors

- Never overshoots: critically damped, so it approaches the target from one side.
- Reaches ~26% of a step after one `tau` and ~95% after `5 * tau`.
- **Frame-rate independent.** Because it is the closed-form solution rather than an Euler integration, twenty 4ms steps land where one 80ms step does, to nine decimal places - a meter on a 144Hz panel and one on a 60Hz panel read the same, and a dropped frame does not change where the dot ends up.
