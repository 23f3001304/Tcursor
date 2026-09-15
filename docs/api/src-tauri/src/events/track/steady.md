# src-tauri/src/events/track/steady.rs

The shape-hold filter both cursor tracks are read through. The OS cursor flips arrow, I-beam, arrow, I-beam every few milliseconds while a pointer crosses a paragraph (each gap between lines is a flip), and every shape has its own hotspot, so a drawn cursor that follows every flip visibly jitters (owner, 2026-09-15). The filter drops any change that did not stay on screen for the hold, and is applied once, in `CursorTrack::load` and `CursorLayer::load`, which the export (`export/cursor/`) and the preview IPC (`cursorpreview::cursor_kinds`, `cursor_layer`) both go through. That is what keeps the two sides drawing the same shape at the same moment without a TypeScript mirror: the preview receives samples that are already steady. The files on disk keep every flip, so a different hold later reads the same recordings differently and nothing is lost at record time.

## SHAPE_HOLD_MS

```rust
pub const SHAPE_HOLD_MS: u32 = 100
```

How long a shape has to stay before the drawn cursor follows it, in recording-clock ms. A real dwell on text or a link lasts far longer, so those still show, one beat late; the flips of a crossing last 15 to 60 ms and never show. A feel knob in the "fake polish" sense, a constant for now.

## steady

```rust
pub fn steady<T: Copy + PartialEq>(samples: &[(u32, T)], hold_ms: u32) -> Vec<(u32, T)>
```

`samples` (sorted by time, one entry per change) with every change that did not last `hold_ms` removed. A sample survives when it is the first (the base the track starts from), the last (nothing after it can prove it short), or when the next sample is at least `hold_ms` later; consecutive equal survivors collapse to the first of them, so a flicker that ends where it started leaves no trace at all, and a flicker that ends on a real dwell keeps that dwell's own start time.

### Behaviors

- `a_flicker_between_two_shapes_is_dropped_and_the_real_dwell_keeps_its_start` - four flips over 90 ms vanish; the I-beam that stays begins at its own sample time.
- `a_flicker_that_ends_where_it_started_leaves_no_trace` - the track is just its base.
- `a_change_that_lasts_exactly_the_hold_is_kept` - the bound is inclusive.
- `the_first_sample_is_the_base_even_when_it_is_short`.
- `empty_and_single_tracks_pass_through`.
- `a_zero_hold_only_collapses_repeats` - the filter degrades to a de-duplicator.

### Used by

- `src-tauri/src/events/track/cursortype.rs` - `CursorTrack::load`.
- `src-tauri/src/events/track/cursorlayer.rs` - `CursorLayer::load`.
