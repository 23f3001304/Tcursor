# src-tauri/src/export/spring.rs

A real damped harmonic oscillator behind `Easing::Spring`, plus its wire form `spring(stiffness,damping[,mass])`. Mirrored in TS by `src/shared/math/spring.ts` (`shared/math/spring.md`); the export is the source of truth and the preview follows it.

**What this replaced.** `camera::ease` used to match `Easing::Spring { .. }` and evaluate a fixed ease-out-back curve (`k = 1.70158`), ignoring both fields - so the wire word "spring" meant one hard-coded shape and the stiffness/damping that `SPRING_DEFAULT` carried were decoration. The CurveEditor's Spring card drew a hand-authored cubic that was not that curve either. Now the fields are the physics, the wire carries them, and the card samples the same function the export evaluates.

### The time-remap semantics (read this before changing a constant)

A spring has no natural duration - it has a *rate*. But every caller here (`ZoomRegion::zoom_in_ms`, `LayoutSeg::transition_ms`, `CameraMove`) already owns the wall-clock duration, and an easing curve is by definition a map from progress `[0,1]` to value. So the curve is defined as:

> the oscillator's unit-step response over **its own settle time**, remapped so `p = 1` is the settle instant.

Settle time is "the slowest decaying mode has fallen to `1e-3`" - `LN_EPS / sigma`, where `sigma` is the decay rate in units of `w0` (see `settle_u`).

**The consequence, and it is a big one:** `w0` cancels. `u = w0 * t` and `T = u*/w0`, so `p * T * w0 = p * u*` - the normalized curve is a function of the **damping ratio alone**. Two springs with the same `zeta = c / (2*sqrt(k*m))` are the same curve; `spring(170,26)` and `spring(680,52)` are indistinguishable (`the_shape_depends_on_the_damping_ratio_alone` pins this). Stiffness and mass still matter, because moving either one alone moves `zeta` - it is only the *combination* that holds `zeta` fixed which is a no-op.

**The trade-off.** A stiffer spring settles faster in *shape*, not in wall time: it spends a larger fraction of `dur_ms` already arrived. A very underdamped spring does the opposite - it fires in the first ~10% of the duration and rings for the rest. That is the honest rendering of those parameters inside a fixed window, but it does mean "make it snappier" is `dur_ms`'s job, not stiffness's. The alternative (let the spring set its own duration and ignore `dur_ms`) was rejected: it would make a zoom's pill length a lie, which the whole ramp-fitting design (`fit_durations`) exists to prevent.

## PREFIX

```rust
pub const PREFIX: &str = "spring(";
```

The wire prefix, mirrored by `SPRING_PREFIX` in `src/shared/math/spring.ts`.

## STIFFNESS

```rust
pub const STIFFNESS: (f32, f32) = (1.0, 2000.0);
```

The `(min, max)` the wire, the ops layer (`valid_easing`) and the UI slider all agree on. Positive by definition - a zero-stiffness spring has no restoring force and no curve.

## DAMPING

```rust
pub const DAMPING: (f32, f32) = (0.0, 200.0);
```

Damping may be **0** - a spring that never stops ringing - which `ZETA_MIN` then makes finite rather than rejecting.

## MASS

```rust
pub const MASS: (f32, f32) = (0.1, 10.0);
```

Positive by definition. On the wire for completeness; the UI does not expose it, because it enters the curve only through `zeta` and is therefore redundant with stiffness for shaping.

## ZETA_MIN

```rust
const ZETA_MIN: f32 = 0.05;
```

Floor on the damping ratio. At `zeta = 0` the oscillator never settles, so there is no window to normalize onto; `0.05` is ~15 visible oscillations, which is as "boing" as this can express without dividing by zero.

## LN_EPS

```rust
const LN_EPS: f32 = 6.907_755;
```

`ln(1/1e-3)` - the settle band. *Why 1e-3 and not the 1% a settling-time definition usually uses:* the residue left at `p = 1` is taken out linearly (see `spring`), and at 1% the critically damped branch's polynomial factor leaves `(1 + ln(100)) * 0.01 = 5.6%` to remove, which is a visible tilt. At 1e-3 the worst case (exactly critical) is `0.77%` and everything else is under `0.11%` - invisible.

## CRIT

```rust
const CRIT: f32 = 1e-3;
```

Half-width of the band around `zeta = 1` evaluated with the critically damped form, where the under- and overdamped ones both divide by ~0. The three expressions agree to ~5e-4 at the boundary.

## zeta

```rust
fn zeta(stiffness: f32, damping: f32, mass: f32) -> f32
```

`c / (2*sqrt(k*m))`, with each input clamped into its published range and the result floored at `ZETA_MIN`. The one number the curve's shape depends on (see the semantics section).

## settle_u

```rust
fn settle_u(z: f32) -> f32
```

`w0 * t` at which the slowest mode has decayed to `1e-3` - the instant `p = 1` maps to.

`LN_EPS / z` when `z <= 1` (the decay rate is `zeta * w0`), `LN_EPS / (z - sqrt(z^2 - 1))` above it (the slow root dominates). **Continuous at `z = 1`**, where both branches give `w0`. *Why not the underdamped envelope's own amplitude factor `1/sqrt(1-z^2)`:* it diverges as `z -> 1`, so a spring a hair under critical would have got a settle time 40% longer than a critically damped one - a visible kink for a parameter change of 0.001.

## resp

```rust
fn resp(z: f32, u: f32) -> f32
```

The unit-step response at `u = w0 * t`, from rest at 0 toward 1 - the textbook three branches:

- **underdamped** (`z < 1`): `1 - e^(-z u) * (cos(d u) + (z/d) sin(d u))`, `d = sqrt(1 - z^2)`. Overshoots past 1 and rings down.
- **critically damped** (`|z - 1| <= CRIT`): `1 - (1 + u) e^(-u)`. The fastest approach with no overshoot.
- **overdamped** (`z > 1`): `1 - (r2 e^(r1 u) - r1 e^(r2 u)) / (r2 - r1)`, `r1,r2 = -(z -/+ sqrt(z^2 - 1))`. Monotone and slow.

`sqrt` arguments are floored at 0 so an `f32` rounding of `z*z - 1` at `z ~ 1` cannot produce NaN.

## spring

```rust
pub fn spring(stiffness: f32, damping: f32, mass: f32, p: f32) -> f32
```

The easing curve itself: `resp(z, p * u*) + p * (1 - resp(z, u*))`.

### Inputs

- `stiffness, damping, mass: f32` - the oscillator, clamped into the published ranges by `zeta`.
- `p: f32` - progress. `<= 0` (and non-finite) returns exactly `0.0`; `>= 1` returns exactly `1.0`, so an out-of-range caller clamps rather than extrapolating.

### Returns

The eased value. **Both endpoints are exact**, not merely close: the second term removes the sub-0.1% residue the settle band leaves at `u*`, linearly in `p`, so the curve still starts at 0, still ends at 1, and the correction (at most 0.77%, at exactly critical damping) is monotone and therefore cannot introduce a wobble. Underdamped parameters overshoot past 1 on the way; critical and overdamped ones are monotone.

## eval

```rust
pub fn eval(e: Easing, p: f32) -> f32
```

The `Easing::Spring` arm `camera::ease` and `export::easing::ease` both delegate to - so there is one spring evaluator, not one per easing table. Any other variant returns `p` unchanged (it is never called with one).

## parse_spring

```rust
pub fn parse_spring(s: &str) -> Option<(f32, f32, f32)>
```

Parses `spring(stiffness,damping)` or `spring(stiffness,damping,mass)`, mass defaulting to `1.0`, each value clamped into its published range. Malformed, non-finite, or wrong-arity input is `None`, so callers fall through to their own default exactly as they do for a bad `cubic(...)`. Two fields or three - never one, never four.

## format_spring

```rust
pub fn format_spring(stiffness: f32, damping: f32, mass: f32) -> String
```

The canonical wire string: fixed 3-decimal fields and always all three, so a value that round-trips through `parse_spring` + this is byte-stable and the frontend's `formatSpring` writes the identical bytes (which is what lets `valid_easing` leave a UI-written value untouched).

### Behaviors worth knowing

- `endpoints_are_exact_for_every_branch` (unit test): `p = 0` is `0.0` and `p = 1` is `1.0` exactly, on all three branches plus the clamped extremes; `p = 0.999` is already within 1e-3, so the endpoint is approached rather than jumped to. Out-of-range and NaN `p` clamp.
- `underdamped_overshoots_and_more_so_the_lower_the_damping_ratio` (unit test): peaks 1.388 (`300/10`, zeta 0.289) > 1.163 (`100/10`, zeta 0.5) > 1.000 (`170/26`, zeta 0.997).
- `critical_and_overdamped_are_monotone` (unit test): 2000 samples never go backwards.
- `the_shape_depends_on_the_damping_ratio_alone` (unit test): `spring(170,26)` == `spring(680,52)` at every sampled `p`.
- `params_are_clamped_and_zero_damping_still_settles` (unit test): damping 0 produces a finite curve that rings past 1.5 rather than a NaN or an infinite window.
- `wire_form_round_trips_and_defaults_mass` (unit test) and `parity_table_matches_the_ts_mirror` (unit test): the second is the contract with `src/shared/math/spring.test.ts` - the same 9 values per parameter set, to 1e-4, on both sides.
- `fromedit_spring_tests.rs`: the wire end to end - `easing_from("spring")` reaching `camera::ease`, a parameterised spring overshooting further than a near-critical one, and a real `LayoutTrack` transition extrapolating past its destination scene by an amount that tracks the damping ratio.
