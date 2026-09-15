# src/editor/motion/keys.ts

TS mirror of the Rust `export::keys` module - the hand-drawn `keys(...)` easing curve the motion editor writes and the export renders. The export is the source of truth; this only drives the live preview and the motion graph, so the two must change together. `keys.test.ts` pins the shared table on this side and `export::keys_tests` pins the same numbers on the other, to 1e-4.

**The wire form.**

```
keys(t v in_dx in_dy out_dx out_dy mode, t v in_dx in_dy out_dx out_dy mode, ...)
```

- 2 to `MAX_KEYS` keys, comma-separated; the fields inside a key are separated by one or more spaces.
- `mode` is one letter - `b` bezier, `h` hold, `l` linear - and describes the segment that STARTS at that key, so the last key's mode is parsed and then ignored.
- `t` is the key's time as a fraction of the ramp: strictly increasing, the first exactly 0 and the last exactly 1. `v` is the ramp's progress there and is unclamped - below 0 is anticipation, above 1 is overshoot.
- `in_dx in_dy` / `out_dx out_dy` are the tangent handles as OFFSETS from the key, in the same normalized units. The first key's `in` and the last key's `out` are never evaluated and are written as `0 0`.

**Why a string and not a `curve: {}` object.** `easing` is the channel every consumer already reads and every mirror already parses, next to the named curves, `cubic(...)` and `spring(...)`. A second field would have to be reconciled with `easing` at every one of those sites; as a new spelling of the same string, the timeline popover, the AI director's ops and the seed all keep working untouched and no doc version bumps (`docs/superpowers/specs/2026-09-15-motion-editor-design.md` 1).

## KeyMode

```ts
export type KeyMode = "b" | "h" | "l"
```

The segment that starts at a key: bezier, hold or linear. One letter on the wire because a curve carries up to eight of them and the string lands in every doc.

## Key

```ts
export interface Key { t: number; v: number; in: [number, number]; out: [number, number]; mode: KeyMode }
```

One key. Mirrors Rust's `Key { t, v, in_, out, mode }`; `in` is spelled out because TS allows it as a property name.

## Keys

```ts
export interface Keys { keys: Key[] }
```

A whole curve, 2 to `MAX_KEYS` keys. Every `Keys` this module HANDS OUT is canonical (`parseKeys`, `canonicalKeys`, `keysToString` and `toKeys` all go through `canonicalKeys`); one the graph editor builds mid-drag is not, which is exactly why the canonicaliser is exported.

## KEYS_PREFIX

```ts
export const KEYS_PREFIX = "keys(";
```

Mirror of Rust's `PREFIX`. The cheap "does this string want to be a curve" test, used by `isKeys` before the real parse.

## MAX_KEYS

```ts
export const MAX_KEYS = 8;
```

Mirror of the fixed `[Key; 8]` array Rust stores, which is what keeps `Easing` `Copy` and keeps `ZoomRegion`, `Kf` and the layout segments the plain values they are today. Eight keys is also as many as a 320px graph can hold apart.

## r3

```ts
const r3 = (x: number) => number
```

3 decimals, with `-0` folded to `0`. Everything in the canonical form goes through it, so the writer can never emit `-0.000` - a byte the Rust side would not produce, and the pins compare strings.

## parseKeys

```ts
export function parseKeys(s: string): Keys | null
```

The strict wire parser, returning the CANONICAL curve. Anything off-form is `null`: a nested or missing paren, fewer than 2 or more than `MAX_KEYS` keys, a key that is not exactly 7 fields, a non-finite number, an unknown mode letter, a first `t` that is not 0, a last `t` that is not 1, or times that do not strictly increase. Endpoints and monotonicity are judged on the ROUNDED times - the ones the canonical form keeps - so a string cannot parse as valid and then round into a degenerate segment.

`null` is not an error: `ease` degrades to `"smooth"`, which is what `valid_easing` would have coerced the string to on the way into the doc anyway. A malformed curve therefore previews as the curve it will be stored as.

## canonicalKeys

```ts
export function canonicalKeys(k: Keys): Keys
```

The one normal form both sides agree on:

- keys sorted by `t` (a graph drag can pull a key past its neighbour; the sort is what makes that legal),
- every number rounded to 3 decimals,
- `out_dx` clamped to `[0, next.t - t]` and `in_dx` clamped to `[-(t - prev.t), 0]`, so `x(s)` stays monotone inside every segment and the bisection below always has exactly one root. The `dy` are NOT clamped - that is where overshoot lives,
- the first key's `in` and the last key's `out` zeroed rather than left as drag litter.

Clamping reads the already-rounded neighbour times, so the clamp limit is itself a 3-decimal number and the written string is a fixed point of a second pass.

## keysToString

```ts
export function keysToString(k: Keys): string
```

The canonical string, byte-identical to Rust's writer: `toFixed(3)` on every field, single spaces inside a key, no space after a comma. Canonicalises first, so it is safe to call on a curve straight out of a drag.

## bez

```ts
function bez(p0: number, p1: number, p2: number, p3: number, s: number): number
```

One cubic Bernstein evaluation with every intermediate through `Math.fround`. The mirror evaluates in f64 and the export in f32; rounding each step keeps the two within the 1e-4 the pins demand instead of letting them drift apart over 20 bisection steps.

## evalKeys

```ts
export function evalKeys(k: Keys, p: number): number
```

The curve's value at ramp progress `p`.

1. `p` is clamped to `[0,1]`; the segment is the `[k_i, k_i+1]` with `k_i.t <= p < k_i+1.t`, the last one also taking `p = 1`.
2. `h` returns `k_i.v` (the plateau holds the value of the key that starts it). `l` lerps `k_i.v -> k_i+1.v` by `(p - k_i.t) / (k_i+1.t - k_i.t)`.
3. `b` is the cubic with `P0 = (k_i.t, k_i.v)`, `P1 = P0 + k_i.out`, `P2 = P3 + k_i+1.in`, `P3 = (k_i+1.t, k_i+1.v)`. `x(s) = p` is solved by plain bisection, exactly 20 iterations and no early exit, so both sides land on the same `s` for the same input; `y(s)` is the answer. This is the same shape of solve `export/cubic.rs` does for CSS cubics, and it is not shared code because here the endpoints are free.

**The result is NOT clamped.** Anticipation below 0 and overshoot above 1 are the whole point of drawing a curve by hand, and every consumer already accepts values outside `[0,1]` because springs overshoot today.

## twoKey

```ts
const twoKey = (x1: number, y1: number, x2: number, y2: number, mode?: KeyMode): Keys
```

A two-key ramp from `(0,0)` to `(1,1)` whose handles are the CSS-style control points `(x1,y1)` and `(x2,y2)` - the shared body of every `toKeys` conversion. A cubic's `P2` is absolute where a key's `in` is relative, hence the `-1`.

## EASE_IN_OUT_H

```ts
const EASE_IN_OUT_H = 0.45;
```

The symmetric handle length whose bezier sits closest to `ease_in_out`'s PIECEWISE quadratic, fitted over 201 samples: max deviation 5.2e-3. It is the one named curve with no exact cubic (a piecewise quadratic has a jump in its second derivative, a cubic does not), and 5e-3 of a ramp is half a pixel of a zoom. Every other named curve converts exactly, to within the 3-decimal rounding of the handles (3e-4).

## toKeys

```ts
export function toKeys(easing: string): Keys | null
```

The keys form of an easing string, so the graph can start dragging a curve that was never in keys form. A `keys(...)` string passes through canonicalised; `linear` becomes two keys in mode `l`; `smooth`, `ease_in` and `ease_out` become the cubic that reproduces them EXACTLY (smoothstep is the cubic with handles `(1/3, 0)` and `(-1/3, 0)`, which is why Soft and `"smooth"` are the same curve); `ease_in_out` uses the fit above; `cubic(x1,y1,x2,y2)` becomes `keys(0 0 0 0 x1 y1 b,1 1 (x2-1) (y2-1) 0 0 b)`, with `parseCubic`'s own x-clamping applied first.

Everything else is `null`: a spring, which has no keys at all and is edited by `SpringControls` instead, and any string `ease` does not know. **Only the wire names `ease` itself understands convert** - the underscore spellings `valid_easing` accepts (`ease_in`, not `ease-in`). Converting a string `ease` treats as smooth would silently change a region's motion the first time the graph touched it.

## isKeys

```ts
export const isKeys = (easing: string | null | undefined) => boolean
```

Does this easing string carry a drawable curve? Prefix test first, then a real parse: a malformed `keys(...)` renders as smooth, so it is NOT one, and the inspector shows the preset row rather than a graph of nothing.
