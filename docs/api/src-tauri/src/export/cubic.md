# src-tauri/src/export/cubic.rs

Custom easing curves as CSS-semantics cubic beziers, and the `cubic(x1,y1,x2,y2)` string they travel as. **This module is the source of truth** - `src/lib/cubicBezier.ts` mirrors it for the live preview and is pinned to the same probe values by tests on both sides.

**Why a string, not a schema change.** `Zoom.easing`, `LayoutSeg.easing` and `CameraMove.easing` are all `String` fields holding a wire-name. A custom curve is just another wire-name, so nothing about the doc schema changes and a doc written before custom curves existed parses byte-identically. It also means `Easing::Cubic` is the ONE variant `easing_from` can reconstruct exactly - the string carries the whole shape, unlike `Spring`, whose stiffness/damping cannot fit.

## PREFIX

```rust
pub const PREFIX: &str = "cubic(";
```

The wire-form prefix, so the parser and any future matcher agree on one literal.

## parse_cubic

```rust
pub fn parse_cubic(s: &str) -> Option<(f32, f32, f32, f32)>
```

Parse `cubic(x1,y1,x2,y2)`. Surrounding and per-field whitespace is tolerated.

### Returns

`Some` control points, or `None` for anything malformed: a missing prefix/suffix, the wrong arity (3 or 5 fields), an unparseable or empty field, or a non-finite value (`inf`/`NaN`).

### Clamping

- `x1`, `x2` are clamped into `[0, 1]`. *Why:* it is exactly what keeps `x(t)` monotonic over the curve, so `solve_t` always has a single answer. Out-of-range x is clamped rather than rejected, so a slightly-off client value still produces a usable curve instead of silently degrading to Smooth.
- `y1`, `y2` are left free. *Why:* overshoot past 1 (or dipping below 0) is the entire point of a "back"/anticipation curve; CSS allows it too.

## format_cubic

```rust
pub fn format_cubic(x1: f32, y1: f32, x2: f32, y2: f32) -> String
```

The canonical wire string: fixed 3-decimal fields (`cubic(0.250,0.100,0.250,1.000)`). Fixed width makes the value byte-stable, so `parse_cubic` -> `format_cubic` is idempotent and a doc does not churn on re-save. The TS `formatCubic` emits the identical text, so a value written by the frontend survives `valid_easing`'s normalisation unchanged.

## eval

```rust
pub fn eval(x1: f32, y1: f32, x2: f32, y2: f32, p: f32) -> f32
```

Evaluate the curve at progress `p` (clamped to `[0,1]`). Cheap enough for per-frame use.

### Implementation

1. `p <= 0` and `p >= 1` short-circuit, so the endpoints are exact by construction rather than by convergence.
2. `solve_t` finds the curve parameter where `x(t) == p`: **Newton** first (typically ~3 iterations), each step clamped back into `[0,1]`, bailing out if the derivative goes near-zero; then **bisection** as the guaranteed fallback, which cannot get stuck because `x(t)` is monotonic for in-range `x1`/`x2`.
3. `bez(y1, y2, t)` returns the value coordinate at that parameter.

`bez`/`dbez` are the Horner forms of `3(1-t)^2*t*a + 3(1-t)*t^2*b + t^3` and its derivative, shared by both the x-solve and the y-read.

### Behaviors worth knowing

- `matches_the_css_ease_curve` - `cubic(.25,.1,.25,1)` IS the CSS `ease` keyword: `0.802403` at `p=0.5`, `0.408511` at `p=0.25`, both within `1e-3`. `cubic(.42,0,.58,1)` (CSS `ease-in-out`) is `0.5` at the midpoint. The `1/3`-`2/3` handles reproduce the identity curve.
- `is_monotonic_in_x_for_in_range_handles` - 101 samples across five degenerate x pairs (including `(1,0)` and `(0,0)`) never run backwards.
- `overshoots_past_one_when_a_handle_does` - `cubic(.34,1.56,.64,1)` exceeds 1 mid-flight and still lands exactly on 1.

### Used by

- `src-tauri/src/export/camera/mod.rs` - `ease`'s `Cubic` arm (the evaluator that actually runs).
- `src-tauri/src/export/easing.rs` - `ease`'s `Cubic` arm.
- `src-tauri/src/export/render/fromedit.rs` - `easing_from` parses the wire string back to `Easing::Cubic`.
- `src-tauri/src/edit/ops/region.rs` - `valid_easing` accepts and canonicalises the wire string.
- `src-tauri/src/edit/seed.rs` - `easing_str` renders `Easing::Cubic` back out.
