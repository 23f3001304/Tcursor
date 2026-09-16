# src-tauri/src/edit/ops/clipops.rs

The clip track's edit ops (`SplitAt`, `MoveClip`, `UpdateClip`, `RemoveClip`), split out of `api.rs` the way `effects.rs`, `captions.rs` and `textops.rs` are. `api::apply` routes all four variants here in one delegating arm. Spec 6.6, under the ruling at the top of spec section 6 (owner, 2026-09-15): the doc-wide clip model was built, not M9's per-clip-owns-its-edits model - `EditDoc.clips` is a reordering of source ranges over ONE shared, doc-wide region set (zooms, cuts, speed, ...), which is what keeps this a batch rather than a milestone. An empty `clips` list means "one clip, the whole trim-resolved recording" and is what every pre-clips document reads as (`clip.md`). Nothing renders `EditDoc.clips` yet (Batch 4) - this module only maintains it.

## MAX_TRANSITION_MS

```rust
pub const MAX_TRANSITION_MS: u32 = 2000;
```

The hard ceiling on a clip's `transition_in_ms`, in milliseconds, before `apply_clip` clamps it further to half the shorter neighbouring clip's OUTPUT length (`clamp_transition`) - so the 2 s cap alone never lets a dissolve outlast the material on either side of it.

## apply_clip

```rust
pub fn apply_clip(doc: &mut EditDoc, op: EditOp)
```

Applies one clip op. Every op first resolves `region::dur_bound(doc)`; on a doc with neither `clip_ms` nor `trim.out_ms` set that is `u32::MAX`, and the WHOLE op is a no-op (the private `full_dur` returns `None`) - nothing can be clamped or resolved into a doc whose length is not yet known.

- `SplitAt { at_ms }` - splits the recording at `at_ms`, CLIP time. With `doc.clips` empty, materialises the trim-resolved range (`doc.trim.resolve(full)`) as two new clips meeting at `at_ms`, ids from `ids::next_clip_id` (`cl0`, `cl1`, ...); a split at or outside either trim edge is a no-op. With clips already present, finds the one clip whose source range STRICTLY contains `at_ms` and divides it in two, inserting the new right-hand half immediately after it; a split at a clip's own edge, or at a point covered by no clip, is a no-op (`position` finds nothing to split). Cuts and speed spans are untouched by a split - they stay in source time and are clipped per clip by `TimeMap` (spec 6.3), so a span straddling the split point takes effect on both sides of it.
- `MoveClip { id, to_index }` - removes the clip and reinserts it at `to_index`, clamped to the list length AFTER the removal, so an out-of-range index moves the clip to the end rather than doing nothing or panicking. An unknown `id` is a no-op.
- `UpdateClip { id, src_in_ms, src_out_ms, transition_in_ms }` - patches only the fields it carries into the matching clip; an unknown `id` is a no-op. `src_in_ms`/`src_out_ms` are each clamped to `[0, full]` where supplied, then the pair is ordered by an unconditional swap if `src_in_ms` ends up greater than `src_out_ms` - unlike `region::clamp_order`, which favours whichever handle the caller just moved, a clip's two edges are symmetric, so there is no "the other one follows" preference to encode. If the clamped-and-ordered range is zero-length, the clip is dropped outright (`doc.clips.remove(i)`), and a `transition_in_ms` supplied in the SAME call is never applied - the clip that would receive it no longer exists. Otherwise, a supplied `transition_in_ms` is clamped by `clamp_transition`: first to `[0, MAX_TRANSITION_MS]`, then to half the OUTPUT length of the shorter of this clip and its predecessor, via a `TimeMap` built fresh from the doc as it stands after the range edit (`TimeMap::clip_out_ms`). On the first clip (`i == 0`) there is no predecessor, so only the flat `MAX_TRANSITION_MS` cap applies - the value is still stored, but the renderer ignores a first clip's `transition_in_ms` (spec 6.6: there is nothing before it to dissolve from). Building a `TimeMap` here is throwaway - a few microseconds even on a long doc - because clip ops are user-interactive, not a hot path.
- `RemoveClip { id }` - drops the clip by id, but only when more than one clip remains; removing the LAST clip is a no-op (`doc.clips.len() > 1` guards the `retain`), so a project can never be edited back down to zero clips. Removing the second-to-last leaves exactly one clip and does NOT normalise back to an empty list - the user asked for clips, so `TimeMap` keeps resolving that one clip from `doc.clips`, not from the empty-list "whole trim" case, even though the two happen to describe the same range right after the removal.

Any other op is a no-op (`_ => {}`) - `api::apply`'s match only routes the four clip variants here, the same shape `textops::apply_text` uses.

### Behaviors

- `the_first_split_materialises_two_clips_over_the_trim` - the first split on a clip-less doc produces `cl0`/`cl1` covering `trim.resolve(full)` on either side of the split point.
- `a_split_at_an_edge_outside_every_clip_or_on_an_unresolvable_doc_is_a_noop` - a split at either trim edge, or past it, is a no-op on an empty `clips` list; splitting the same point twice only ever produces two clips, never three; a split on a doc with neither `clip_ms` nor `trim.out_ms` set leaves `clips` empty.
- `a_second_split_divides_the_clip_containing_it_and_keeps_the_order` - splitting again inside the first of two clips inserts the new clip between them (`cl0`, `cl2`, `cl1`), not at the end.
- `a_split_inherits_the_cuts_and_speed_spans_that_straddle_it` - a cut and a speed span straddling the split point both survive as one entry each and take effect on both sides, pinned against `TimeMap::clip_out_ms`.
- `move_clip_reorders_and_clamps_the_index` - moving a clip to index `0` reorders it to the front; an out-of-range `to_index` clamps to the end; an unknown id is a no-op.
- `update_clip_clamps_and_orders_the_source_range_and_drops_a_zero_length_clip` - `src_out_ms` clamps to `full`; `src_in_ms > src_out_ms` in the same call swaps to the ordered pair; a range collapsed to zero length drops the clip.
- `a_transition_is_clamped_to_two_seconds_and_half_the_shorter_neighbour` - a 5 s request clamps to half a 1 s neighbour (500 ms), then to `MAX_TRANSITION_MS` once the neighbour is long enough, then to half its OUTPUT length once an 8x speed span shrinks that neighbour to 875 ms of output (437, floored); a first clip's transition is stored but only ever flat-capped.
- `remove_clip_never_removes_the_last_one_and_never_normalises_back_to_empty` - removing one of two clips leaves the other; removing that last one is a no-op.
- `the_four_ops_parse_from_their_wire_shapes` - all four ops round-trip through their `#[serde(tag = "op")]` wire shapes.
