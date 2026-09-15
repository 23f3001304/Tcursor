# src/shared/math/remap.ts

TS mirror of the Rust `export::remap` module (`docs/api/src-tauri/src/export/remap.md`): the same algorithms in the same expression order, f64 on both sides and no `fround`, so the two clocks agree to the bit. The export is the source of truth; `remap.test.ts` pins the parity table computed from the Rust fixture (`remap.fixture.ts`). Clip time is the raw clip's clock (0 = the first video frame, every lane's pills, the media elements' `currentTime`); output time is the exported file's clock, the one every region is evaluated on (`remapDoc`). Plain functions over a frozen `TimeMap` value, the codebase's idiom.

## FACTOR_MIN

```ts
export const FACTOR_MIN = 0.25;
```

## FACTOR_MAX

```ts
export const FACTOR_MAX = 8;
```

## Segment

```ts
export interface Segment { clipStart: number; clipEnd: number; factor: number; outStart: number }
```

One kept range `[clipStart, clipEnd)` of clip time, the factor it plays at, the output time it starts at.

## TimeMap

```ts
export interface TimeMap { readonly segments: Segment[]; readonly outDur: number; readonly trimIn: number; readonly plain: boolean }
```

Built once per doc change (`useTimeMap`); every reader below is pure and cheap per tick. `plain` is true with no cuts and no speed spans (a trim may exist): every clock function is then the identity on kept time and the transport hides the clip-time fine print.

## resolveTrim

```ts
export function resolveTrim(trim: Trim, fullDurMs: number): [number, number]
```

Mirrors Rust `Trim::resolve`: `out_ms === 0` means the whole clip; `in` is clamped to `out`.

## buildTimeMap

```ts
export function buildTimeMap(trim: Trim, cuts: Cut[], speed: Speed[], fullDurMs: number): TimeMap
```

Trim first, then cuts clamped into it, sorted and merged when overlapping or touching, then speed spans clamped, sorted, a later span clamped to start at its predecessor's end, factors clamped to `[FACTOR_MIN, FACTOR_MAX]`; every kept piece split at span edges and tagged with the span's factor (1 outside). A cut inside a span removes those frames from it. Mirrors `TimeMap::build`.

## identityMap

```ts
export const identityMap = (fullDurMs: number): TimeMap
```

## outDurMs

```ts
export const outDurMs = (m: TimeMap): number
```

The exported length, rounded: the transport's total and the export dialog's length.

## outOf

```ts
export function outOf(m: TimeMap, clipMs: number): number
```

Output time of a clip time; inside a gap, the next segment's start (the frame the viewer sees next); past the end, the total.

## clipOf

```ts
export function clipOf(m: TimeMap, outMs: number): number
```

The inverse on the kept ranges; at or past the end, the last kept edge.

## gapContaining

```ts
export function gapContaining(m: TimeMap, clipMs: number): [number, number] | null
```

The gap a clip time falls in as `[previous kept end, next kept start]`, the trailing gap running to `Infinity` (Rust: `u32::MAX`); `null` on a kept range. The preview jumps the media over a finite gap and tells a cut jump from a seek with it.

## factorAt

```ts
export function factorAt(m: TimeMap, clipMs: number): number
```

The factor of the segment containing a clip time, 1 elsewhere; the preview's `playbackRate`.

## frameBounds

```ts
export function frameBounds(m: TimeMap, i: number, fps: number): [number, number] | null
```

Inclusive recording-frame bounds of segment `i`: the first segment floors its start and the last floors its end (today's trim semantics), a cut edge is exact.

## framePlan

```ts
export function framePlan(m: TimeMap, fps: number): number[]
```

For every output frame, the recording frame it shows. Not used by the preview itself (the media run on clip time); kept so the parity table can pin the plan on both sides.
