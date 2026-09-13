# src/lib/spring.ts

TS mirror of the Rust `export::spring` module - the same damped-harmonic-oscillator math, because the export is the source of truth and this only drives the live preview and the curve cards. Every change here needs the matching change there (and vice versa); `spring.test.ts` pins the shared parity table on this side and `export::spring_tests::parity_table_matches_the_ts_mirror` pins it on the other.

**The semantics are documented once**, in `docs/api/src-tauri/src/export/spring.md` - in particular *why* `p` is remapped onto the spring's own settle time, and the consequence that the drawn shape depends on the damping ratio alone. Do not restate the reasoning here; it will drift.

## SPRING_PREFIX

```ts
export const SPRING_PREFIX = "spring(";
```

Mirror of Rust's `PREFIX`.

## SPRING_RANGE

```ts
export const SPRING_RANGE = { stiffness: [1, 2000], damping: [0, 200], mass: [0.1, 10] } as const;
```

Mirrors Rust's `STIFFNESS`/`DAMPING`/`MASS` consts. What `SpringControls` bounds its sliders with, so the UI cannot author a value the wire would clamp.

## SPRING_DEFAULT

```ts
export const SPRING_DEFAULT: [number, number, number] = [100, 10, 1];
```

Mirror of Rust's `types::SPRING_DEFAULT` - what the bare wire word `"spring"` means (Motion's own default spring, `zeta = 0.5`).

## zeta

```ts
function zeta(stiffness: number, damping: number, mass: number): number
```

`c / (2*sqrt(k*m))`, inputs clamped and result floored at `ZETA_MIN` (0.05). Mirror of Rust's `zeta`.

## settleU

```ts
const settleU = (z: number) => number
```

`w0 * t` at which the slowest mode has decayed to 1e-3 - the instant `p = 1` maps to. Mirror of Rust's `settle_u`, including the continuity at `z = 1`.

## resp

```ts
function resp(z: number, u: number): number
```

The unit-step response at `u = w0 * t`, three branches (underdamped / critical / overdamped). Mirror of Rust's `resp`.

## spring

```ts
export function spring(stiffness: number, damping: number, mass: number, p: number): number
```

The easing curve. Exact `0` at `p <= 0` (and for a non-finite `p`) and exact `1` at `p >= 1`; underdamped parameters overshoot past 1 on the way, critical and overdamped ones are monotone.

### Used by

- `src/editor/timeline/layoutTrack.ts` - `ease` dispatches every spring wire-name here, so the preview's layout blends match the export's.
- `src/editor/inspectors/curves.ts` - `springPath` samples this to draw the Spring card, so the card cannot lie about the curve.

## parseSpring

```ts
export function parseSpring(s: string | undefined | null): [number, number, number] | null
```

Parses `spring(stiffness,damping)` or `spring(stiffness,damping,mass)`, mass defaulting to 1, values clamped into `SPRING_RANGE`. Malformed or wrong-arity input is `null`. Note the **bare word `"spring"` is not a match** - it has no parens; use `springOf` when you want either form.

## formatSpring

```ts
export const formatSpring = (stiffness: number, damping: number, mass = 1) => string
```

The canonical wire string, byte-identical to Rust's `format_spring` (fixed 3-decimal fields, always all three). *Why byte-identical matters:* `valid_easing` re-emits what it parses, so a value this writes survives the round trip through the backend unchanged - otherwise every slider drag would come back as a different string and the inspector would re-render.

## springOf

```ts
export function springOf(easing: string | undefined | null): [number, number, number] | null
```

The parameters an easing wire-name means, or `null` when it is not a spring at all: the bare word `"spring"` resolves to `SPRING_DEFAULT`, `spring(...)` carries its own. The single place the UI and `ease` both ask "is this a spring, and which one" - so a new spring spelling only has to be taught here.

### Behaviors worth knowing

- `matches the Rust parity table to 1e-4` (unit test): the contract with the Rust side, four parameter sets x nine values of `p`. Rust computes in `f32` and this in `f64`; the closed form has no iteration, so they agree well inside the tolerance.
- `has exact endpoints on every branch`, `overshoots more the lower the damping ratio, and not at all past critical`, `is monotone when critically damped or overdamped`, `depends on the damping ratio alone, so the same zeta is the same curve` (unit tests) - the same four properties `spring_tests.rs` asserts, so a divergence fails on both sides.
- `round-trips and defaults mass`, `clamps out-of-range values and rejects malformed input`, `resolves the bare word to SPRING_DEFAULT and leaves other curves alone` (unit tests).
