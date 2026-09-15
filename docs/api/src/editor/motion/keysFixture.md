# src/editor/motion/keysFixture.ts

The keys-curve parity table, shared by `keys.test.ts` (evaluation) and `keysWire.test.ts` (the wire format). Split out when the one file outgrew the test budget: both halves need the same six curves, and a second copy of the table is exactly the drift the parity test exists to catch.

## P

```ts
export const P: number[]
```

The seven progress samples the Rust side's own table is measured at: `0, 0.1, 0.25, 0.5, 0.75, 0.9, 1`. `CURVES[i].want[n]` is the value at `P[n]`.

## CURVES

```ts
export const CURVES: { name: string; wire: string; canon: string; want: number[] }[]
```

Six curves - snappy, cinematic in, cinematic out, mechanical, soft, hand-made - each in the loose wire form a caller may write, in the canonical form `keysToString` must round-trip to byte for byte, and with its values at `P`. `CURVES[4]` (soft) is the one that must draw today's `smoothstep`, and `CURVES[5]` (hand-made) is the one with anticipation and overshoot, so the evaluator's clamp is on `p` and not on the value.

## parsed

```ts
export const parsed = (s: string) => Keys
```

`parseKeys` plus the assertion that it did not return `null`, so a case that is only interested in the curve does not have to unwrap by hand.

## key

```ts
export const key = (t: number, v: number, out?: [number, number], inn?: [number, number]) => Key
```

A bare bezier key at `(t, v)` with optional handles - what the hand-built `canonicalKeys` cases are assembled from.
