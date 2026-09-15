# src/shared/math/cubicBezier.ts

TS mirror of the Rust `export::cubic` module. The export is the source of truth; this exists only so the live preview evaluates a custom curve the same way the exported frame will. Any change here needs the matching change there (and vice versa) - `cubicBezier.test.ts` pins the shared probe values on this side, `cubic_tests.rs` on the other.

**Where it is used from.** `layoutTrack.ts`'s `ease` is the ONLY place easing is evaluated in TypeScript, and it feeds both consumers: the layout cross-fade (`layoutAt`) and the camera-keyframe blend (`camMoveAt`). Zoom easing is *not* evaluated here at all - the zoom camera curve arrives pre-baked from Rust as `camera_track`, so a custom zoom curve is already exact in the preview with no mirror needed.

## evalCubic

```ts
export function evalCubic(x1: number, y1: number, x2: number, y2: number, p: number): number
```

Evaluate a CSS-semantics cubic bezier (P0=(0,0), P1, P2, P3=(1,1)) at progress `p`, clamped to `[0,1]`. Endpoints short-circuit, so `0` and `1` are exact rather than converged. Internally: Newton to find the curve parameter where `x(t) === p` (clamped each step, bailing on a near-zero derivative), then bisection as the guaranteed fallback - safe because `x(t)` is monotonic for in-range `x1`/`x2`.

## parseCubic

```ts
export function parseCubic(s: string | undefined | null): [number, number, number, number] | null
```

Parse the `cubic(x1,y1,x2,y2)` wire form, tolerating whitespace. `null` for anything malformed - wrong arity, empty or unparseable field, non-finite value, or a nested paren. Accepts `null`/`undefined` directly so callers can pass a possibly-absent easing without a guard.

`x1`/`x2` clamp to `[0,1]` (keeps `x(t)` monotonic); `y1`/`y2` stay free so a curve can overshoot. Identical rules to the Rust parser, and the rejection set is pinned by the same test list on both sides.

## formatCubic

```ts
export const formatCubic: (x1: number, y1: number, x2: number, y2: number) => string
```

The canonical wire string, byte-identical to Rust's `format_cubic` (fixed 3-decimal fields). Writing this exact form means the value the frontend commits survives `valid_easing`'s normalisation unchanged, so the inspector does not see its own value rewritten under it after a round trip.
