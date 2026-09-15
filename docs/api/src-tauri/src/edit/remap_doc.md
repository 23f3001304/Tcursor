# src-tauri/src/edit/remap_doc.rs

Every region list of an `EditDoc` moved onto the OUTPUT clock through a `TimeMap` (`export/remap.md`), so the renderer and the preview evaluate zooms, layouts, effects and camera moves in the time the viewer sees. `render_edit::EditState::load` runs the doc through this before `fromedit` builds any track, which is why no evaluator changed for the time remap. Mirrored by `src/shared/math/remapDoc.ts` (`docs/api/src/shared/math/remapDoc.md`).

## remap_doc

```rust
pub fn remap_doc(doc: &EditDoc, map: &TimeMap) -> EditDoc
```

A clone of `doc` with: `zooms`, `layout`, `effects` and `captions` mapped `start_ms`/`end_ms` through `map.out_of`, a region whose mapped span collapses (entirely inside a cut) dropped; `camera_moves` mapped `t_ms` (a keyframe pinned to a removed moment lands on the cut's end, the frame the viewer sees next); every duration (`zoom_in_ms`, `zoom_out_ms`, `transition_ms`, `transition_out_ms`, `fade_in_ms`, `fade_out_ms`) untouched, which is the whole point of approach A in the design (a 350 ms zoom-in stays 350 ms of output inside a 2x span); ids untouched; `trim` reset, `cuts` and `speed` emptied (consumed), `clip_ms` set to `map.out_dur_ms()`; `settings` and `aspect` as they were.

Captions carry one thing the other lists do not: each caption's `words` have their OWN timings, and they are mapped through the same `span` helper. Without that the line would move but the word-by-word highlight would drift off the syllable inside a speed span, and a word inside a cut would light at the wrong moment. A word whose span collapses is dropped from the caption, exactly as a region that collapses is dropped from its list.

### Behaviours

- `regions_move_to_the_output_clock_and_keep_their_durations` - a zoom at clip 2200..3200 on the parity fixture lands at 700..1350 with its 350/450 ms ramps intact.
- `a_region_entirely_inside_a_cut_is_dropped_and_one_straddling_it_shrinks`.
- `camera_moves_map_their_time_and_the_consumed_fields_are_cleared`.
- `a_plain_map_is_the_identity_on_regions`.
