# src-tauri/src/edit/ops/timeops.rs

The cut and speed-span ops, kept apart from `api.rs` (at the size cap) and normalised after every write so the doc never holds overlapping cuts, overlapping speed spans, out-of-range factors or empty ranges. `api::apply` calls `apply_time_op` first and only falls through to its own match when the op is not one of these seven. The rendering side of the same data is `export::remap::TimeMap` (`export/remap.md`), which re-normalises defensively; the two agree by construction.

## next_cut_id

```rust
pub(crate) fn next_cut_id(doc: &EditDoc) -> String
```

`c{n}`, `n` one past the highest numeric suffix among the doc's cut ids (the count when none parse). The same strategy `next_zoom_id` uses.

## next_speed_id

```rust
pub(crate) fn next_speed_id(doc: &EditDoc) -> String
```

`s{n}`, moved here from `api.rs` unchanged.

## normalize_cuts

```rust
pub fn normalize_cuts(doc: &mut EditDoc)
```

Clamp every cut into `[0, region::dur_bound(doc)]`, drop empty ones, sort by start, merge overlapping or touching neighbours into the earlier one (its id survives, its end grows).

## normalize_speed

```rust
pub fn normalize_speed(doc: &mut EditDoc)
```

Clamp every span into the clip and its factor into `[remap::FACTOR_MIN, remap::FACTOR_MAX]`, sort by start, clamp a span to start at its predecessor's end (spans never overlap; the earlier span wins the contested stretch), drop empty ones.

## apply_time_op

```rust
pub fn apply_time_op(doc: &mut EditDoc, op: &EditOp) -> bool
```

`true` after handling one of: `AddCut` (fresh id, then normalise), `AddCuts` (the incoming spans are sorted and merged FIRST, then given ids in time order, then normalised against the existing cuts, so a Remove silences batch lands with stable ids whatever order the detector listed them in), `UpdateCut` and `UpdateSpeed` (only the `Some` fields, through `region::clamp_order` so a partial update can never invert a range; a zero-length result is dropped by the normalisation), `RemoveCut`, `SetSpeed` (fresh id, normalise), `RemoveSpeed`. `false` for every other op, untouched.

### Behaviours

- `add_cuts_is_one_op_that_lands_sorted_merged_and_with_ids` - `[(5000,6000),(1000,2000),(1900,2500)]` becomes `c0 = 1000..2500`, `c1 = 5000..6000`.
- `add_cut_merges_into_an_existing_cut_and_keeps_the_earlier_id` - a touching cut merges into `c0`.
- `update_and_remove_cut_by_id_and_a_zero_length_result_is_dropped`.
- `cuts_are_clamped_into_the_clip` - past the clip end is clamped; entirely past it is dropped.
- `speed_spans_never_overlap_and_the_factor_is_clamped` - 40 becomes 8, 0.1 becomes 0.25, the later span starts where the earlier ends, and growing the earlier one pushes the later one's start.
- `a_doc_saved_before_cut_ids_loads_with_ids_assigned` - `EditDoc::assign_missing_ids` numbers the empties past the highest live id.
