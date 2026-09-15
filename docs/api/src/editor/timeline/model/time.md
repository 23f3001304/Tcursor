# src/editor/timeline/model/time.ts

Shared time-formatting and ruler utilities for the editor UI: `fmt` (milliseconds to "M:SS"), `fmtPrecise` (milliseconds to "M:SS.s"), and `rulerTicks` (adaptive ruler tick times + labels across a duration).

## fmt

```ts
export function fmt(ms: number): string
```

Formats a millisecond value as `"M:SS"`.

### Inputs

- `ms: number` - time in milliseconds. Values below zero are clamped to zero via `Math.max(0, ...)`.

### Returns

A string in the form `"M:SS"` where SS is always two digits (zero-padded). For example, `fmt(75000)` returns `"1:15"` and `fmt(0)` returns `"0:00"`. Used by Transport for the dimmed total half of the time readout, and by `ExportProgress`'s ETA text and a HUD settings slider's value label.

## fmtPrecise

```ts
export function fmtPrecise(ms: number): string
```

Formats a millisecond value as `"M:SS.s"` - `fmt`'s one-decisecond sibling, added for design/premium-pass D3's transport readout (the live playhead position reads to a tenth of a second; the dimmed total next to it still uses plain `fmt`). Delegates to the same private `tlabel(ms, dec)` helper `rulerTicks` already uses for its own sub-second tick labels, rather than a second parallel formatter.

### Inputs

- `ms: number` - time in milliseconds. Values below zero are clamped to zero (via `tlabel`).

### Returns

A string in the form `"M:SS.s"`, e.g. `fmtPrecise(12400)` returns `"0:12.4"`. Used by `Transport`'s `.e-time` current-position span. Fix round 1 (design/premium-pass D3 review): `fmtPrecise` is the first caller to feed `tlabel` a continuous, un-quantized `ms` (`rulerTicks`' own inputs are always exact multiples of its tick step, so they never hit this), which could round a seconds remainder like `59.96` up to `"60.0"` after minutes had already been floored from the un-rounded total - e.g. `fmtPrecise(59960)` used to read `"0:60.0"`. `tlabel` now rounds the seconds remainder to display precision FIRST and carries into the minute (`m += 1, s = 0`) if that rounds all the way up to 60, so `fmtPrecise(59960)` now correctly returns `"1:00.0"`. Covered by `time.test.ts`.

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
