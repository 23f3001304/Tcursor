# src/shared/math/remap.fixture.ts

The parity fixture shared by `remap.test.ts`, `remapClips.test.ts`, `remapDoc.test.ts` and `stage/transport/playback.test.ts`. Test-only; not imported by app code.

## fixtureParts

```ts
export const fixtureParts = () => ({ trim, cuts, speed }) as const
```

The base fixture's inputs, so the two maps below share one source of truth (Rust: `remap_tests.rs::fixture_parts`). Trim 500..9000 of a 10000 ms clip, cuts [1000,2000) and [4000,4500), 2x on [2500,3500), 0.5x on [6000,8000). Spread the arrays (`[...p.cuts]`) at the call: they are `as const`.

## fixtureMap

```ts
export function fixtureMap(): TimeMap
```

The Rust fixture (`export/remap_tests.rs::fixture`) rebuilt in TS: `fixtureParts` with NO clips. Every expected value in those tests was computed from this exact map on the Rust side; a TS result that disagrees is wrong, not the table.

## fractionalFixtureMap

```ts
export function fractionalFixtureMap(): TimeMap
```

The map whose cut starts at a FRACTIONAL output time, for `playback.test.ts`: no trim on a 10000 ms clip, one 1.5x span on [0,2000), one cut [3000,4000). Segments `(0, 2000, 1.5, 0)`, `(2000, 3000, 1, 1333.33)`, `(4000, 10000, 1, 2333.33)`; `out_of(3000) = 2333` (rounded DOWN from 2333.33) and `clip_of(2333) = 3000`, the rounded round trip that used to strand the preview at the cut's start instead of seeking to 4000. Rust has no counterpart: `walk_plan` never seeks.

## clipsFixtureMap

```ts
export function clipsFixtureMap(): TimeMap
```

The clips fixture (`export/remap_clips_tests.rs::clips_fixture`): the same cuts and spans, two clips in REVERSE source order, the trim ignored. Its pinned table, computed from the spec and checked by hand:

```
clips (output order): A = (6000, 9000), B = (500, 4000); same cuts (1000-2000, 4000-4500),
same spans (2500-3500 x2, 6000-8000 x0.5), full 10_000, trim ignored once clips exist.

segments (clip, clip_start, clip_end, factor, out_start):
  (0, 6000, 8000, 0.5, 0)      (0, 8000, 9000, 1.0, 4000)
  (1,  500, 1000, 1.0, 5000)   (1, 2000, 2500, 1.0, 5500)   (1, 2500, 3500, 2.0, 6000)   (1, 3500, 4000, 1.0, 6500)
out_dur 7000, plain false, clip_out_ms(0) = 5000, clip_out_ms(1) = 2000

out_of:  0->5000  500->5000  750->5250  1500->5500  2250->5750  3000->6250  3750->6750
         4000->0  4250->0  6000->0  7000->2000  8000->4000  8500->4500  9000->7000  9999->7000
clip_of: 0->6000  1000->6500  4000->8000  4500->8500  5000->500  5250->750  5500->2000
         6000->2500  6250->3000  6500->3500  6750->3750  7000->4000
frame_plan(10): [60,60,61,61,...,78,78,79] (39 entries: 60..79 each twice, minus the last),
                80..89, 5..9, 20..24, 25,27,29,31,33, 35..40   -> NOT monotone
crosses_boundary: (3999,4000) false (8000==8000 contiguous)  (4999,5000) true  (5499,5500) true
                  (5999,6000) false  (6499,6500) false  (100,101) false
base fixture crosses_boundary: (499,500) true  (999,1000) false  (1499,1500) false
                  (1999,2000) true  (3499,3500) false  (7499,7500) false  (0,0) false
```
