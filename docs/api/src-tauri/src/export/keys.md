# src-tauri/src/export/keys.rs

Keyframed easing: the `keys(...)` wire form the motion editor writes, its canonical writer, and the evaluator behind `Easing::Keys`. **This module is the source of truth** - `src/editor/motion/keys.ts` mirrors it for the live preview and both sides are pinned to the same literal values (`keys_tests.rs` / `keys.test.ts`, tolerance 1e-4).

**Why a string, not a new schema field.** `Zoom.easing`, `LayoutSeg.easing` and `CameraMove.easing` are already `String` wire-names carrying `smooth`, `cubic(...)` and `spring(...)`. A keyframed curve is just another wire-name, so no doc version bumps, every existing doc reads unchanged, and the timeline popover, the AI director's ops and the seed keep working untouched. A parallel `curve: {keys}` object would have to be reconciled with `easing` at every one of those sites.

**The form.**

```
keys(t v in_dx in_dy out_dx out_dy mode, t v in_dx in_dy out_dx out_dy mode, ...)
```

2 to 8 keys, comma-separated; the seven fields inside a key are separated by one or more spaces. `t` and `v` are the key's time and value as fractions of the ramp - `t` strictly increasing with the first key at 0 and the last at 1, `v` unclamped so a curve can anticipate below 0 or overshoot past 1. The two handles are OFFSETS from the key, not absolute points, so dragging a key carries its handles along. `mode` is one letter describing the segment that STARTS at that key: `b` bezier, `h` hold, `l` linear. The last key's mode is parsed and then ignored, because no segment starts there.

## PREFIX

```rust
pub const PREFIX: &str = "keys("
```

The wire-form prefix, so the parser and the writer agree on one literal (the same shape `cubic::PREFIX` and `spring::PREFIX` take).

## MAX

```rust
pub const MAX: usize = 8
```

Keys per curve. *Why a cap at all:* it is what lets `Keys` be a fixed-size `Copy` struct, so `Easing` stays `Copy` and `ZoomRegion`, `CameraMoveTrack`'s `Kf` and `LayoutTrack`'s segments all keep their shape with no allocation anywhere in the per-frame path. 8 is also the graph editor's own limit - past that the dots stop being individually draggable at 320 px wide.

## KeyMode

```rust
pub enum KeyMode { Bezier, Hold, Linear }
```

What the segment starting at a key does. `Hold` is what makes a plateau expressible (Mechanical's "constant speed, then a short hold"); `Linear` gives a truly constant rate, which a bezier with zero handles only approximates.

## Key

```rust
pub struct Key { pub t: f32, pub v: f32, pub in_: [f32; 2], pub out: [f32; 2], pub mode: KeyMode }
```

One keyframe. `in_` is `[in_dx, in_dy]` and `out` is `[out_dx, out_dy]`, both offsets from `(t, v)`. `in_` is spelled with the trailing underscore because `in` is a Rust keyword; the wire form and the TS mirror both spell it `in`.

## Keys

```rust
pub struct Keys { pub n: u8, pub keys: [Key; MAX] }
```

A whole curve, fixed-size so it is `Copy`. Only the first `n` entries are meaningful - always read them through `slice()` rather than indexing `keys` directly, or the padding will be treated as real keyframes at `t = 0`.

## Keys::slice

```rust
pub fn slice(&self) -> &[Key]
```

The live keys and never the padding. Defensively `min(MAX)` so a hand-built `Keys` with a bad `n` cannot panic.

## parse_keys

```rust
pub fn parse_keys(s: &str) -> Option<Keys>
```

Parse the wire form. Surrounding whitespace and runs of spaces between fields are tolerated.

### Returns

`Some` a curve that is **already canonical**, or `None` for anything the form cannot carry: a missing prefix or closing paren, fewer than 2 or more than 8 keys, the wrong field count in any key, an unparseable or non-finite number, an unknown mode letter, keys whose times are not strictly increasing, or a first/last key not at 0/1. `None` is how a curve degrades: `valid_easing` stores `"smooth"` and `easing_from` returns the config's easing, the rule every other malformed wire-name already follows.

### Normalisation, in this order

1. **Sort by `t`.** *Why:* a drag can carry one key past another, and sorting lands a valid curve instead of throwing the user's edit away.
2. **Validate**, comparing times rounded to 3 decimals (`r3`). *Why round:* the endpoints are checked against the precision the wire actually carries, so a value that WRITES as `1.000` is not rejected for being `0.9999999` after a float round-trip.
3. **Clamp the handles' x offsets**: `out_dx` into `[0, next.t - t]`, `in_dx` into `[-(t - prev.t), 0]`. *Why:* it is exactly what keeps `x(s)` monotone inside every bezier segment, so the solve in `bez_seg` has a single answer. Clamping rather than rejecting means a slightly-off client value still produces a usable curve. `in_dy`/`out_dy` are left free - vertical overshoot is the point.
4. **Zero the first key's `in` and the last key's `out`**, which describe nothing.

Because all of this happens at parse time, `format_keys` is pure formatting and the evaluator can assume a well-formed curve.

## format_keys

```rust
pub fn format_keys(k: &Keys) -> String
```

The canonical wire string: every number at fixed 3 decimals, fields space-separated, keys comma-separated with **no space after the comma**, e.g.

```
keys(0.000 0.000 0.000 0.000 0.333 0.000 b,1.000 1.000 -0.333 0.000 0.000 0.000 b)
```

Fixed width makes the value byte-stable, so `parse_keys` -> `format_keys` is idempotent, a doc does not churn on re-save, and a curve written by the frontend survives `valid_easing`'s normalisation unchanged. `-0.000` (what clamping a small negative handle to zero leaves behind, since `(-0.0001).clamp(-x, 0.0)` keeps the sign) is normalised to `0.000` - without that, the two sides could disagree on a byte for a curve they agree on numerically.

## eval

```rust
pub fn eval(k: &Keys, p: f32) -> f32
```

Evaluate the curve at progress `p`. Cheap enough for per-frame use: one linear scan over at most 8 keys plus 20 bisection steps.

### Implementation

1. `p` is clamped to `[0, 1]`. The **result is not clamped** - anticipation below 0 and overshoot past 1 are the whole point, and every consumer already accepts them (springs overshoot today).
2. Find the segment with `k_i.t <= p < k_{i+1}.t`; the last segment also takes `p = 1`. A curve with fewer than 2 live keys returns that key's value (or `p` when empty) rather than panicking.
3. `Hold` returns `k_i.v`. `Linear` lerps by `(p - k_i.t) / (k_{i+1}.t - k_i.t)`.
4. `Bezier` builds the cubic `P0 = (k_i.t, k_i.v)`, `P1 = P0 + k_i.out`, `P2 = P3 + k_{i+1}.in`, `P3 = (k_{i+1}.t, k_{i+1}.v)`, solves `x(s) = p`, and returns `y(s)`.

### Why bisection and not Newton

`cubic.rs` solves the same shape of problem with Newton first and bisection as a fallback, because there the endpoints are fixed at `(0,0)` and `(1,1)` and speed matters. Here the requirement is different: the TypeScript mirror has to produce the **same** number, and a hybrid solver's iteration count depends on float details that differ between f32 and JS doubles. So `bez_seg` runs a fixed **20 halvings** with no early exit, `lo = 0`, `hi = 1`, `s = (lo + hi) / 2` each round, `lo = s` when `x(s) < p` else `hi = s`, returning `y` at the last midpoint. 20 halvings leaves a residual under 1e-6 in `s`, comfortably inside the 1e-4 tolerance the cross-side pins assert. `cubic_at` is written in plain Bernstein form (not Horner) for the same reason: the mirror evaluates the identical expression.

The residual is also why `eval(curve, 0.0)` is `0.000002` rather than exactly 0 for a curve with a steep `out` handle. Nothing depends on an exact endpoint here - a ramp's start value is already whatever `v` says it is, and the curve may deliberately not end at 1.

### Behaviors worth knowing

- `eval_is_pinned_at_fixed_instants` - the five presets plus one hand-made curve, at p in {0, 0.1, 0.25, 0.5, 0.75, 0.9, 1}, against literals shared with the TS side.
- `soft_matches_smoothstep` - the Soft preset (`0.333`/`-0.333` handles) is `3p^2 - 2p^3` to within 1e-3 across 101 samples. *Why it matters:* Soft replaces the bare word `"smooth"` as the project default, so existing regions must look identical after the switch.
- `handles_are_clamped_into_their_own_segment`, `unsorted_keys_are_sorted_by_time`, `negative_zero_is_written_as_plain_zero` - the canonical form.
- `malformed_curves_are_unparseable` - 9 keys, a first `t` of 0.1, a last `t` of 0.9, a bad mode letter, wrong field counts, a repeated `t`, a single key, a missing paren, and a `cubic(...)` string all return `None`.
- `input_is_clamped_and_output_is_not` - `eval(k, 7.0) == eval(k, 1.0)`, and the hand-made curve reads 1.111 at p = 0.9.
- `both_ease_entry_points_route_to_this_evaluator` - `export::easing::ease` and `export::camera::ease` agree on a `Keys` curve (unlike `Smooth`, where they historically differ).

### Used by

- `src-tauri/src/export/camera/mod.rs` - `ease`'s `Keys` arm (the evaluator the export and the preview actually run).
- `src-tauri/src/export/easing.rs` - `ease`'s `Keys` arm.
- `src-tauri/src/export/render/fromedit.rs` - `easing_from` parses the wire string back to `Easing::Keys`.
- `src-tauri/src/edit/ops/region.rs` - `valid_easing` accepts and canonicalises the wire string on the way into the doc.
- `src-tauri/src/edit/seed.rs` - `easing_str` renders `Easing::Keys` back out.
