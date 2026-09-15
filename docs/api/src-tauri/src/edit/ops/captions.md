# src-tauri/src/edit/ops/captions.rs

The caption track's edit ops, split out of `api.rs` the way `effects.rs` and `timeops.rs` are (M5 T4). `api::apply` routes all six caption variants here in one delegating arm.

The track is an ordinary region list: it clamps through `region::{clamp_order, dur_bound}` like every other lane, and every mutating arm ends with a `sort_by_key(|c| c.start_ms)`, so the timeline lane, the renderer and the "merge with the next one" op can all assume time order without sorting what they were handed. Times are OUTPUT-clock ms, the doc's one-clock contract (`edit::model::EditDoc`).

## MIN_CAPTION_MS

```rust
pub const MIN_CAPTION_MS: u32 = 200;
```

The shortest caption a drag may leave behind. `region::clamp_order` alone collapses an inverted region to zero width, which is right for a zoom (a zero-width zoom is simply inert) but wrong for a caption: a zero-width pill has nothing left to grab on the timeline and nothing to show in the export, so it would be an edit the user could neither see nor undo by dragging. A handle dragged past its partner therefore lands 200 ms away instead of on top of it.

## split_text

```rust
pub fn split_text(text: &str, frac: f32) -> (String, String)
```

Cuts `text` at the ASCII space whose byte index is NEAREST to `frac * text.len()`, ties going to the earlier space (`min_by_key` over an ascending iterator keeps the first minimum), both halves trimmed. A word is never cut in half.

A text with NO space returns `(text, "")` rather than refusing. The empty half becomes a caption the user types into, which reads on the timeline as "the split happened, now fill this in"; a silent no-op would read as a broken Split button.

## split_caption

```rust
pub fn split_caption(c: &Caption, at_ms: u32, next_id: &str) -> Option<(Caption, Caption)>
```

Cuts one caption in two at `at_ms`, which must be STRICTLY inside it - splitting exactly on either boundary would produce an empty half, so `None` (which the op reads as a no-op) is the answer for `at_ms <= start_ms` or `at_ms >= end_ms`.

Words go to the half they START in (`w.start_ms < at_ms`), so a word straddling the cut stays whole on the left rather than being duplicated or dropped, and each half's text is its own words rejoined. A caption with NO word timings (typed by hand, or itself the product of an earlier split) has nothing to partition, so its text is cut by `split_text` at the fraction `at_ms` falls at in the span instead.

The left half keeps `c.id`, so the timeline selection stays on the pill the user was already looking at; the right half takes `next_id` (`ids::next_caption_id`).

## merge_captions

```rust
pub fn merge_captions(a: &Caption, b: &Caption) -> Caption
```

Joins two captions: `a`'s id, the outer span, the two texts joined by one space (an empty side contributes nothing rather than a leading or trailing space), and the two word lists concatenated so the merged line keeps its word-by-word highlight.

## apply_caption

```rust
pub fn apply_caption(doc: &mut EditDoc, op: EditOp)
```

Applies one caption op, then re-sorts the track:

- `UpdateCaption { id, start_ms?, end_ms?, text? }` patches only the fields it carries (the convention every update op follows), clamps each to `dur_bound(doc)`, runs `region::clamp_order` and then the `MIN_CAPTION_MS` floor above. An edited `text` keeps `words` ONLY when it still equals those words joined by single spaces; anything else clears them, because a highlight driven by stale timings lights the WRONG word, which reads as a bug where no highlight at all reads as a caption that simply has none.
- `RemoveCaption { id }` / `ClearCaptions` drop one caption / the whole track.
- `SetCaptions { captions }` replaces the track outright. What `asr::commands::transcribe_project` writes (plan ADDED-8: the backend writes the doc itself and the frontend re-fetches, so no caption array ever crosses IPC in the other direction).
- `MergeCaptions { id }` joins the caption with the one after it in time order; a no-op on the last caption, which has nothing to join.
- `SplitCaption { id, at_ms }` replaces the caption with `split_caption`'s two halves in place.

An unknown id, a merge at the end of the track and an illegal split all return WITHOUT the trailing sort, so a no-op op leaves the doc bit-identical rather than silently reordering equal-start captions. Any non-caption variant is unreachable by contract (`api::apply`'s match only routes these six here), the same shape `effects::apply_effect` uses.

### Behaviors

- `update_retimes_a_caption_and_keeps_start_before_end` - a partial start move retimes; a start dragged past the end takes the end with it and still leaves a positive span.
- `update_clamps_to_the_clip_and_ignores_an_unknown_id` - an end of 99 s on a 20 s clip lands on 20 s; an unknown id changes nothing at all.
- `editing_the_text_keeps_the_word_timings_it_still_matches_and_drops_them_when_it_does_not` - the two halves of the `words` rule above.
- `remove_and_clear_do_what_they_say`, `set_captions_replaces_the_whole_track_and_leaves_it_sorted`.
- `merge_joins_a_caption_with_the_next_one_in_time` - id, span, joined text and 5 words; `merging_the_last_caption_is_a_no_op`.
- `split_cuts_at_the_playhead_and_hands_each_half_its_own_words`, `split_outside_the_caption_is_a_no_op`, `splitting_a_caption_with_no_word_timings_cuts_its_text_at_a_space`.
- `split_text_never_cuts_inside_a_word_and_never_returns_an_empty_half` - including the no-space case.
- `the_track_stays_sorted_by_start_after_every_op`.
