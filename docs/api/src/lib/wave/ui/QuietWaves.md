# src/lib/wave/ui/QuietWaves.tsx

The two waves that mark "nothing is happening yet, but something is about to": the small idle wave that replaced the app's spinner, and the wide horizon wave that carries its empty and first-run states. They share one drift loop and one breathing dot, which is why they share a file.

## useDrift

```ts
function useDrift(path, dot, dotX, s: Drift, seconds: number, reduced: boolean): void
```

Internal. The drift loop both quiet waves share: writes the sine's `d` and parks the dot **on** the wave at `dotX` every frame, or draws the pair once and stops under reduced motion. The dot's y is read back from the same `sineY` that drew the stroke, so it can never float off the line - the two cannot drift apart because there is only one source for both.

## IdleWave

```tsx
export function IdleWave({ size }: { size?: number }): JSX.Element
```

The app's generic in-progress indicator: a small low-amplitude wave with the breathing dot. `Spin` is now a one-line wrapper around this, so every `<Spin>` in the app inherits the wave motif without a call-site change - the brand mark, not a rotating loader ring, is what the app shows while it is working (panel-design benchmark (c) item 2: nobody in the category connects their mark to their busy states).

### Inputs

- `size` - the wave's height in px; the width is `1.7 * size`, so it reads as a wave rather than as a dot at the sizes `Spin` is used at (15-20px, inline beside text).

### Behaviors

- The waveform drifts slowly (`IDLE_DRIFT_S`). The benchmark's idle wave is the *shape*; the drift is what keeps a busy indicator from reading as frozen, which for a spinner replacement would be a regression in state honesty.
- The dot breathes on the same 2s cycle the idle meter uses (`BREATH_PERIOD_S` / `BREATH_MAX`, imported from `voiceWave.ts` so there is one breath in the app).
- Under reduced motion: one still frame, no drift, no breath.

## HorizonWave

```tsx
export function HorizonWave({ w, h }: { w?: number; h?: number }): JSX.Element
```

The empty and first-run state: a very-low-amplitude horizon wave (period `HORIZON_PERIOD_S`, ~4s) in the wave motif's own blue, with the dot flying in from off the left edge and landing on it - so a blank stage has a beginning instead of just being blank (panel-design benchmark (c) item 6: empty states are inert everywhere in the category).

### Behaviors

- The arrival is ~600ms and overshoots its landing point by 8% of the travel before settling, expressed as Motion keyframes with `times` rather than a spring, so the overshoot is exactly the specified 8% rather than whatever a stiffness/damping pair happens to produce.
- The dot lands at 0.62 of the width - the golden section of the horizon, not its centre.
- Under reduced motion the dot is simply already there: `initial={false}` and a zero-duration transition.

### Styling

`.w-horizon` in `src/lib/wave.css` is the only place the `--e-wave` blue appears. That is deliberate: an empty state has no content and no interactive accent to compete with, which is the whole reason the motif can afford a second hue there and nowhere else.

### Used by

- `src/editor/stage/StageEmpty.tsx` - the stage's "Preparing preview".
