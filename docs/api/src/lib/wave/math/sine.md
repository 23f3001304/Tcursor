# src/lib/wave/math/sine.ts

True-sine geometry, shared by every wave the app draws - the recording meter, the processing sweep, the idle and horizon waves. One sampler, so a "wave" in this app is always literally `y = A sin(kx + p)` and never a hand-tuned bezier that only looks like one, and so the shape is testable without a DOM (`sine.test.ts`).

## TAU

```ts
export const TAU: number
```

A full turn, `2 * Math.PI`. Phase arithmetic elsewhere in the motif spells periods as fractions of this rather than re-deriving `2 * Math.PI` per call site.

## SineSpec

```ts
export interface SineSpec {
  w: number; mid: number; amp: number; lambda: number; phase: number;
  step?: number; envelope?: (t: number) => number;
}
```

Everything a sine needs, in px and radians.

- `w` - width the path spans (x runs `0..w`).
- `mid` - y of the midline. SVG y grows downward, so a crest sits at `mid - amp`.
- `amp` - peak height above the midline, i.e. half the peak-to-peak height.
- `lambda` - spatial period in px. Independent of the temporal period, which enters through `phase`.
- `phase` - radians. The animation clock always enters here, never as a transform on the element, so a wave's shape and its motion are the same number.
- `step` - sampling step in px, default 2 (smooth at any DPI the app ships on).
- `envelope` - optional amplitude multiplier called with the normalised x (`0..1`). Used by the sweep to taper the wave behind its scanning head.

## sineY

```ts
export function sineY(spec: SineSpec, x: number): number
```

The wave's y at `x`, in the same px space as `spec.mid`.

### Behaviors

- Zero amplitude gives exactly `spec.mid`, for every x.
- A crest (`mid - amp`) lands a quarter period in at phase 0; a trough (`mid + amp`) three quarters in.
- Exactly periodic in `lambda`.
- `envelope` scales the amplitude only, never the midline, so a fully damped wave is a flat line on the midline rather than a line that has drifted.

## sinePath

```ts
export function sinePath(spec: SineSpec): string
```

An SVG `d` for the sine, sampled every `step` px across `0..w`.

### Returns

`M x y L x y L ...`, with two decimals per coordinate. The last sample always lands exactly on `w` whatever the step divides into, so the stroke reaches the right edge rather than stopping a few px short of it.

## crestBefore

```ts
export function crestBefore(spec: SineSpec, before: number): number
```

The x of the last crest at or before `before`.

### Returns

A real crest of the same wave, within one `lambda` of the limit. Exported for callers that want a dot to sit on an actual crest rather than at an arbitrary phase; it steps back by exactly one period as the limit crosses a crest.
