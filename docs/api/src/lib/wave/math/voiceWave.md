# src/lib/wave/math/voiceWave.ts

The recording meter's voice wave, as pure math: the layer table, the window that tapers the wave into the flat line at both ends, the level's ballistics, and the closed mirrored outline each layer draws. `VoiceWave.tsx` owns only a rAF loop and four `setAttribute` calls; everything that could be wrong - the log mapping, the attack/release, the idle breath, the reduced-motion freeze, the taper - lives here. Pure, DOM-free, frame-rate independent. Tests: `voiceWave.test.ts`.

Replaces `meterFrame.ts`, which drew two open sine strokes and a needle dot.

## Layer

```ts
export interface Layer { phase: number; freq: number; amp: number; drift: number; alpha: number }
```

One drawn layer of the wave.

- `phase` - its constant phase offset, radians.
- `freq` - multiplies the base wavelength `LAMBDA_PX`.
- `amp` - multiplies the live half-amplitude.
- `drift` - multiplies the base phase speed, and its **sign** is the direction that layer travels.
- `alpha` - the layer's fill opacity.

## LAYERS

```ts
export const LAYERS: readonly Layer[]   // 4 entries
```

The four layers, back to front.

### Behaviors

- **Array order is paint order.** The widest and most opaque layer (`amp` 1.00, `alpha` 0.58) is painted first and sits underneath; the thinner, more transparent ones ride over it.
- **One colour, four fills.** Every layer is filled from the same blue-to-violet ramp, so a crossing is darker purely because two translucent fills composited there. Nothing paints a third colour to fake depth.
- **Neighbouring `drift` signs alternate**, so the layers sweep *through* each other instead of sliding along in formation - which is the only reason there is more than one of them.
- The four `freq` values (0.86 / 1.13 / 1.37 / 1.58) are deliberately not ratios of small integers: they share no common period inside the meter's width, so the crossings never land in the same place twice and the shape does not visibly loop.

## LAMBDA_PX

```ts
export const LAMBDA_PX: number   // 58
```

Base spatial period, px - the wavelength of a layer whose `freq` is 1.

## DRIFT_S

```ts
export const DRIFT_S: number   // 2.4
```

Seconds for a layer whose `drift` is 1 to advance one full turn of phase.

## TAPER

```ts
export const TAPER: number   // 0.62
```

Fraction of the width spent on the two cosine tapers, half at each end. The middle is left flat, so the wave stays legible across most of a 104px meter instead of bulging in the centre the way a plain Hann window would.

## AMP_FLOOR_PX

```ts
export const AMP_FLOOR_PX: number   // AMP_MIN / 2
```

Half-amplitude at silence, px. Derived from `level.ts`'s peak-to-peak pair rather than restated, since the lens is mirrored about the midline and its half-height is that peak-to-peak height halved.

## AMP_CEIL_PX

```ts
export const AMP_CEIL_PX: number   // AMP_MAX / 2
```

Half-amplitude at full level, px, in the 30px slot the wave was designed for. `ceilFor` raises it for a taller slot.

## ceilFor

```ts
export function ceilFor(h: number): number
```

The half-amplitude ceiling for a slot `h` px tall: `h / 2 - 3` (3px of air above and below at full level), never less than `AMP_CEIL_PX`, which is exactly this at 30px. The take pill grew to a 40px slot (2026-09-14) and a wave still capped at 12px looked timid in it; at 40px the ceiling is 17.

## AMP_ATTACK_S

```ts
export const AMP_ATTACK_S: number   // 0.02
```

Lag constant of the amplitude follower while it is rising. `damp` is critically damped and reaches ~95% of a step in 4.74 tau, so this is a ~95ms rise: a syllable lands at once.

## AMP_RELEASE_S

```ts
export const AMP_RELEASE_S: number   // 0.065
```

Lag constant while falling - a ~300ms decay, slow enough to read as a decay rather than a flicker.

## MAX_DT_S

```ts
export const MAX_DT_S: number   // 0.25
```

A frame longer than this (a suspended window, a stalled main thread) is treated as this long, so the wave resumes from where it was instead of teleporting.

## IDLE_AFTER_S

```ts
export const IDLE_AFTER_S: number   // 1.2
```

Continuous silence before the wave starts breathing instead of lying dead flat. A pause between sentences is shorter than this, so the breath is not a tell that speech stopped.

## BREATH_PERIOD_S

```ts
export const BREATH_PERIOD_S: number   // 2
```

The breath cycle, shared by the wave's silent swell and `QuietWaves.tsx`'s dot. One cycle, so the two cannot be tuned apart.

## IDLE_SWELL_PX

```ts
export const IDLE_SWELL_PX: number   // 0.6
```

How far above the floor the idle breath swells, px. Kept inside the reference's 1-2px window, so a silent meter reads as "listening" and never as a second signal.

## BREATH_MAX

```ts
export const BREATH_MAX: number   // 1.06
```

Peak scale of the brand dot's own breathe - the mark's idle tell, used by `QuietWaves.tsx`. It lives here beside the wave's breath because the two share `BREATH_PERIOD_S`.

## SAMPLE_PX

```ts
export const SAMPLE_PX: number   // 2
```

Sampling step of the outline, px.

## taperWindow

```ts
export function taperWindow(t: number, taper?: number): number
```

The amplitude envelope across the meter, for a normalised x (0..1).

### Returns

A cosine-tapered (Tukey) window: `0` at both edges, rising over the first `taper / 2` of the width, flat at `1` across the middle, falling again at the far edge. Input is clamped to 0..1.

### Behaviors

- **Zero at the ends is the point.** It is what lets the wave dissolve into the centre line instead of being chopped off at the meter's border, and it is why the end dots always sit on an unbroken line.
- Symmetric about the centre, and monotone through each taper.

## VoiceState

```ts
export interface VoiceState {
  t: number;
  amp: number; ampV: number;
  quietS: number;
  reduced: boolean;
}
```

- `t` - phase clock, seconds. Frozen at `0` under reduced motion.
- `amp` / `ampV` - half-amplitude of the widest layer in px, and its follower velocity.
- `quietS` - seconds of continuous silence seen.

## initialVoiceState

```ts
export function initialVoiceState(reduced: boolean): VoiceState
```

A wave that has just mounted: flat, silent, and **not** yet breathing, so a take that starts loud never shows the idle swell first.

## idleAmp

```ts
export function idleAmp(t: number): number
```

The silent wave's half-amplitude at clock `t`: `AMP_FLOOR_PX` plus a slow cosine swell of `IDLE_SWELL_PX`. Never reaches the amplitude any real signal would produce.

## voiceFrame

```ts
export function voiceFrame(s: VoiceState, rmsMic: number, rmsSys: number, dt: number, ceilPx?: number): VoiceState
```

Advance the wave by `dt` seconds given the latest mic and system RMS (both 0..1). Pure: the caller keeps the returned state and hands it back next frame. `ceilPx` is the half-amplitude a full level reaches - `ceilFor` of the slot height, `AMP_CEIL_PX` when omitted.

### Behaviors

- **One amplitude drives all four layers**, taken from whichever source is louder. The meter answers "how loud is what this take is recording"; a second amplitude would only ask the viewer to tell two overlapping translucent shapes apart at 30px tall, which nobody can do.
- Log-mapped through `level.ts`'s `levelFromRms` over its speech window (`FLOOR_DB`..`CEIL_DB`), then scaled between `AMP_FLOOR_PX` and `ceilPx`, so each halving of loudness costs the same number of px and ordinary speech fills most of the slot.
- Attack and release are the same `damp` with two different lag constants, picked per frame by which way the level is moving.
- After `IDLE_AFTER_S` of continuous silence the target becomes `idleAmp`; the first real sample resets `quietS` to `0` in the same frame.
- Under reduced motion the clock stays `0` and the amplitude snaps straight to its target with no velocity - the level is still reported, but nothing drifts.
- `dt` is clamped to `MAX_DT_S`.

## layerSpec

```ts
export function layerSpec(s: VoiceState, layer: Layer, w: number, h: number): SineSpec
```

The sine one layer is riding this frame, in the meter's own px space. A plain `SineSpec` from `sine.ts`, with `taperWindow` as its envelope, so the wave is literally `y = A sin(kx + p)` under a window and not a hand-tuned bezier.

## lensPath

```ts
export function lensPath(spec: SineSpec): string
```

A closed SVG `d` for one layer: the sine across the width, then its mirror image back again, joined into a single fillable shape.

### Behaviors

- **Mirroring rather than filling down to the midline is what produces the lens shapes.** The two halves meet wherever the sine crosses zero, so a layer is a chain of pinched lobes above and below the line, and at the two ends the taper closes it to a point on the line itself.
- Every sample is equidistant from `spec.mid`, and the first and last points land on it whatever the level.

## layerPath

```ts
export function layerPath(s: VoiceState, layer: Layer, w: number, h: number): string
```

`lensPath` of `layerSpec` - the one call a frame makes per layer.
