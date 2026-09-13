# src-tauri/src/edit/ops/region.rs

Small shared helpers for placing, bounding and validating timeline regions (zooms, effects, layout segments and camera keyframes alike), split out of `api.rs` so that file stays under the size limit.

## dur_bound

```rust
pub(crate) fn dur_bound(doc: &EditDoc) -> u32
```

The document's known upper time bound for PLACING a new or moved region. `clip_ms` (the true recording length) wins when known, so trimming the clip does not collapse a region added past the trim point; `trim.out_ms` is the fallback for a doc predating `clip_ms`; `u32::MAX` is the last resort for a not-yet-seeded doc (which `edit::seed` should always prevent).

## clamp_order

```rust
pub(crate) fn clamp_order(start: &mut u32, end: &mut u32, start_was_set: bool)
```

After a partial `start_ms`/`end_ms` update leaves a region inverted (`*start > *end`), pulls the field the caller did NOT just set to match the one they did, so an inversion never persists to disk. `start_was_set` picks the winner: `true` pulls `end` up to `start` (the caller just dragged the start handle past the end); `false` pulls `start` down to `end`. No-op when already ordered.

**Why (M5):** `UpdateZoom`, `UpdateLayoutSeg`, `UpdateEffect` each clamp `start_ms`/`end_ms` independently against the clip's upper bound but never related the two fields to each other - a client sending `start_ms: 8000` on a region whose `end_ms` is 5000 used to persist `{start: 8000, end: 5000}` verbatim. Both real consumers (`CameraSim::winner`, `spotlight_sim`) are underflow-safe against this (an inverted range is simply never selected), so it never crashed - but the region kept rendering as a live, editable timeline pill that silently did nothing, with no way for the user to tell why.

### Behaviors

- `clamp_order_pulls_end_to_a_start_dragged_past_it` - `(8000, 5000)` with `start_was_set=true` -> `(8000, 8000)`.
- `clamp_order_pulls_start_to_an_end_dragged_before_it` - `(5000, 1000)` with `start_was_set=false` -> `(1000, 1000)`.
- `clamp_order_is_a_noop_when_already_ordered` - `(100, 200)` is unchanged either way.

### Used by

- `src-tauri/src/edit/ops/api.rs` - `UpdateZoom`, `UpdateLayoutSeg`
- `src-tauri/src/edit/ops/effects.rs` - `UpdateEffect`

## auto_layer

```rust
pub(crate) fn auto_layer(existing: &[(u32, u32, u32)], start_ms: u32, end_ms: u32) -> u32
```

Assigns a NEW region to the lowest layer (0, 1, 2, ...) with nothing already overlapping `[start_ms, end_ms)` on it. Existing layers are read as-is and never reassigned, so this can never disturb a layer the user set by hand. `existing` is `(start_ms, end_ms, layer)` per region - zoom and effect regions both flow through this same rule.

## valid_layout

```rust
pub(crate) fn valid_layout(s: &str) -> String
```

Coerces a layout preset wire-name to one of `screen` / `camera` / `presenter` / `screen_only` / `camera_only`; anything else becomes `"screen"` (which is also the "empty means default" fallback the layout track uses for gaps).

## valid_easing

```rust
pub(crate) fn valid_easing(s: &str) -> String
```

Coerces an easing wire-name to something `easing_from` can actually reconstruct, so a typo'd, stale or hostile string can never reach the renderer.

### Accepted

1. One of the six named curves (`linear`, `smooth`, `spring`, `ease_in`, `ease_out`, `ease_in_out`) - returned verbatim.
2. A well-formed `spring(stiffness,damping[,mass])` - re-emitted through `export::spring::format_spring`, which **canonicalises** the text (fixed 3 decimals, always all three fields, so the stored value is byte-stable and the frontend's `formatSpring` writes the identical bytes), **fills in the default mass**, and **applies the range clamps** from `parse_spring`. `spring(99999,-4,50)` is stored as `spring(2000.000,0.000,10.000)`, not rejected.
3. A well-formed custom `cubic(x1,y1,x2,y2)` - re-emitted through `export::cubic::format_cubic`, same canonicalise-and-clamp deal (the x-clamp from `parse_cubic`). A client that sends `cubic(-1,0.5,2,0.5)` gets `cubic(0.000,0.500,1.000,0.500)` stored.
4. Anything else degrades to `"smooth"` - the tuned default, matching what the TS `ease` mirror falls through to for an unparseable name. Note the bare word `"spring"` is arm 1, not arm 2: it stays a bare word, and `easing_from` resolves it to `SPRING_DEFAULT`.

### Used by

- `src-tauri/src/edit/ops/api.rs` - `UpdateZoom`, `UpdateLayoutSeg` and `UpdateCameraMove` all run their `easing` through it; `AddLayoutSeg` runs its `layout` through `valid_layout`.

### Behaviors worth knowing

- `valid_easing_keeps_named_curves_and_canonicalises_cubics` - pins all four arms, including both clamps, the default mass, and the `cubic(1,2)` / `spring(170)` arity failures degrading to `"smooth"`.
- `auto_layer_finds_lowest_free_layer` / `auto_layer_reuses_a_free_layer_that_does_not_overlap`.
