# src/lib/wave/math/sweep.ts

The processing and export wave: one sine sweeping left to right with the dot as its scanning head. Indeterminate work (the AI director) drives the head from a clock; an export drives it from percent complete, and the whole wave's amplitude decays to flat as that percent reaches 100. Pure, tested in `sweep.test.ts`, consumed by `SweepWave.tsx`.

## SWEEP_PERIOD_S

```ts
export const SWEEP_PERIOD_S: number   // 1.8
```

Seconds for one indeterminate left-to-right pass.

## TRAIL

```ts
export const TRAIL: number   // 0.34
```

How much of the width the wave trails behind the head, as a fraction of that width.

## sweepHead

```ts
export function sweepHead(t: number, period?: number): number
```

The head's position (0..1) at `t` seconds.

### Returns

A sawtooth: the pass restarts at the left edge rather than bouncing back, because a bounce reads as "undoing work". Stays inside `0..1` for a negative clock, and returns `0` for a zero or negative period rather than dividing by it.

## sweepEnvelope

```ts
export function sweepEnvelope(x: number, head: number, trail?: number): number
```

The amplitude envelope at normalised `x` for a head at `head`.

### Returns

`1` right behind the head, fading to `0` one `trail` further back, and exactly `0` ahead of the head - the wave only exists where work has already passed. The falloff is a raised cosine, so the trailing edge dies out smoothly instead of ending on a visible corner, and it is monotone the whole way.

## progressAmp

```ts
export function progressAmp(pct: number): number
```

The global amplitude multiplier for an export at `pct` percent: `1` at the start, exactly `0` at 100. Clamped, so a stale or out-of-range percent can never invert the decay or overshoot it.

## headFor

```ts
export function headFor(t: number, pct: number | undefined): number
```

Where the head sits for a given frame.

### Returns

`sweepHead(t)` when `pct` is `undefined` (indeterminate work), otherwise `pct / 100` clamped to `0..1` - so for an export the dot **is** the percentage marker rather than a second indicator that has to agree with it.
