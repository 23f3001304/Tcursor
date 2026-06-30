# src/editor/time.ts

Shared time-formatting and ruler utilities for the editor UI: `fmt` (milliseconds to "M:SS") and `rulerTicks` (adaptive ruler tick times + labels across a duration).

## fmt

```ts
export function fmt(ms: number): string
```

Formats a millisecond value as `"M:SS"`.

### Inputs

- `ms: number` - time in milliseconds. Values below zero are clamped to zero via `Math.max(0, ...)`.

### Returns

A string in the form `"M:SS"` where SS is always two digits (zero-padded). For example, `fmt(75000)` returns `"1:15"` and `fmt(0)` returns `"0:00"`. Used by Transport for the time readout.

## Tick

```ts
export interface Tick { at: number; label: string }
```

One ruler tick: its time position `at` (ms) and its display `label`.

## rulerTicks

```ts
export function rulerTicks(dur: number): Tick[]
```

Adaptive ruler ticks at "nice" intervals so labels never repeat - a 2s clip gets 0.5s ticks, a 5-minute clip gets 30s ticks.

### Inputs

- `dur: number` - total clip duration in milliseconds.

### Returns

An array of `Tick` (`{ at, label }`). It picks the smallest "nice" step from `[200ms .. 600s]` that is `>= dur / 6`, so the ruler shows roughly 6 labels regardless of duration; sub-second steps get a one-decimal label so adjacent ticks stay distinct. For `dur <= 0` it returns a single `0:00` tick. The Timeline positions each tick at `(at / dur) * 100%`.
