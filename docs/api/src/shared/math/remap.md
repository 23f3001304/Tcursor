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
export interface Segment { clipStart: number; clipEnd: number; factor: number; outStart: number; clip: number }
```

One kept range `[clipStart, clipEnd)` of clip time, the factor it plays at, the output time it starts at, and `clip`, the index of the clip it came from in the clip list AS WRITTEN, so `clips[s.clip]` is always the clip that produced it. A dropped clip (inverted or empty) leaves a gap in the indices carried rather than renumbering the ones after it; 0 with no clips, the trim range being index 0. Ordered in clip time within one clip only; across a clip join it can jump anywhere.

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
export function buildTimeMap(trim: Trim, cuts: Cut[], speed: Speed[], clips: Clip[], fullDurMs: number): TimeMap
```

Trim first, then cuts clamped into it, sorted and merged when overlapping or touching, then speed spans clamped, sorted, a later span clamped to start at its predecessor's end, factors clamped to `[FACTOR_MIN, FACTOR_MAX]`; every kept piece split at span edges and tagged with the span's factor (1 outside). A cut inside a span removes those frames from it. Mirrors `TimeMap::build`.

That is the body of an outer loop over the clip ranges (`clipRanges`, which returns `[index, a, b][]`): no clips means one range, the resolved trim at index 0, and every pinned table is unchanged; otherwise one range per clip in OUTPUT order, `[src_in_ms, src_out_ms]` clamped to `fullDurMs` with inverted and empty ranges dropped and the trim ignored. Cuts and speed spans stay in source time and resolve inside each range; `outStart` is declared before the loop and accumulates across the clips, so the output is the concatenation of each clip's kept pieces. Each segment carries the range's OWN index, not the loop's position, so dropping a clip does not renumber the ones after it and `clips[s.clip]` stays valid. `plain` compares the bounds only: it needs the ranges to be exactly the resolved trim AND no cut or span in any of them.

## identityMap

```ts
export const identityMap = (fullDurMs: number): TimeMap
```

## outDurMs

```ts
export const outDurMs = (m: TimeMap): number
```

The exported length, rounded: the transport's total and the export dialog's length.

## nextShown

```ts
export function nextShown(m: TimeMap, clipMs: number): Segment | undefined
```

The segment that shows the next kept source instant after a clip time: the smallest `clipStart` strictly greater than `clipMs`, ties broken by the smaller `outStart`; `undefined` when nothing after it is kept. The second pass of `outOf`, exported because `playbackAction` needs the SEGMENT, not its rounded output time: a seek target has to be an exact `clipStart`, and `clipOf(outOf(t))` rounds through the output clock, which on a fractional `outStart` lands back inside the cut.

## outOf

```ts
export function outOf(m: TimeMap, clipMs: number): number
```

Output time of a clip time: the first showing of that source instant. Two passes, since clips leave the segments unsorted in clip time - the segment containing it, else `nextShown` (the smallest `clipStart` greater than it, ties: the smaller `outStart`), else the total. With one clip in source order that is the old "next segment", which is why the trim/cut/speed tables hold.

## clipOf

```ts
export function clipOf(m: TimeMap, outMs: number): number
```

The inverse on the kept ranges; at or past the end, the last kept edge.

## crossesBoundary

```ts
export function crossesBoundary(m: TimeMap, prevOutMs: number, outMs: number): boolean
```

Whether two OUTPUT times are separated by a splice: their segments (`segOfOut`, the first whose output range has not ended) are `a < b` and some join between them has `clipEnd !== clipStart`. A cut and a clip join count; a speed-span edge, a contiguous clip join, a backwards step and a pair inside one segment do not, and neither does a trim edge (`segOfOut` is -1 outside the mapped range, so the trim's edges are the ends of the output rather than interior joins). Mirrors `TimeMap::crosses_boundary`, which replaced `gapContaining`/`crosses_cut` on both sides. The predicate for a caller holding two output times and no plan; a caller walking `framePlan` uses `planBoundaries` instead.

## clipOutMs

```ts
export function clipOutMs(m: TimeMap, clip: number): number
```

The output length of one clip: the rounded sum of `(clipEnd - clipStart) / factor` over the segments with that index, 0 for an index with no segments - including an index whose clip was dropped, which contributes nothing to the output. The index is the position in the clip list as written, the same one the caller holds.

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

`TimeMap::plan_boundaries`, the last function of the Rust module, mirrors into `remapPlan.ts` rather than this file: `remap.ts` is at the 200-line budget and the plan walk is the one reader that is about the PLAN rather than the clocks.
