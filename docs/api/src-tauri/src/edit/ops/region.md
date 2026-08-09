# src-tauri/src/edit/ops/region.rs

Small shared helpers for placing, bounding and validating timeline regions (zooms, effects, layout segments and camera keyframes alike), split out of `api.rs` so that file stays under the size limit.

## dur_bound

```rust
pub(crate) fn dur_bound(doc: &EditDoc) -> u32
```

The document's known upper time bound for PLACING a new or moved region. `clip_ms` (the true recording length) wins when known, so trimming the clip does not collapse a region added past the trim point; `trim.out_ms` is the fallback for a doc predating `clip_ms`; `u32::MAX` is the last resort for a not-yet-seeded doc (which `edit::seed` should always prevent).

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
2. A well-formed custom `cubic(x1,y1,x2,y2)` - re-emitted through `export::cubic::format_cubic`, which both **canonicalises** the text (fixed 3 decimals, so the stored value is byte-stable) and **applies the x-clamp** from `parse_cubic`. A client that sends `cubic(-1,0.5,2,0.5)` gets `cubic(0.000,0.500,1.000,0.500)` stored, not a rejection.
3. Anything else degrades to `"smooth"` - the tuned default, matching what the TS `ease` mirror falls through to for an unparseable name.

### Used by

- `src-tauri/src/edit/ops/api.rs` - `UpdateZoom`, `UpdateLayoutSeg` and `UpdateCameraMove` all run their `easing` through it; `AddLayoutSeg` runs its `layout` through `valid_layout`.

### Behaviors worth knowing

- `valid_easing_keeps_named_curves_and_canonicalises_cubics` - pins all three arms, including the clamp and the `cubic(1,2)` arity failure degrading to `"smooth"`.
- `auto_layer_finds_lowest_free_layer` / `auto_layer_reuses_a_free_layer_that_does_not_overlap`.
