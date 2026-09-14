# src-tauri/src/edit/ops/smart_zoom.rs

Smart typing duration for a manual zoom (owner, 2026-09-14: "when typing is happening the zoom duration adjusts accordingly"). A zoom marked `smart_typing` has its `end_ms` refitted by `edit::commands::apply_edit_op` whenever it is switched on or its start moves: the end lands one hold after the last key of the typing chain that begins at (or just after) the start, so the zoom stays on the field for as long as the words keep coming and lets go once they stop. The doc always carries a concrete `end_ms`, so the timeline, the preview and the export need no new path - this is a fit, not a live mode. The frontend's switch is the Zoom inspector's Timing row (`durationOptions`).

## smart_end

```rust
pub fn smart_end(start_ms: u32, keys: &[u32], hold_ms: u32, dur_ms: u32) -> Option<u32>
```

Where a smart zoom starting at `start_ms` ends, given ASCENDING keystroke times on the same clock: the first key within `hold_ms` of the start opens a chain, each further key within `hold_ms` of the previous extends it, and the end is the last key plus `hold_ms`, capped at `dur_ms` and never before `start_ms + 1`. `None` when no key falls within `hold_ms` of the start (the zoom keeps its own end); keys before the start never count. `hold_ms` is `ZoomSettings::hold_ms`, the same idle release the auto-zoom's smart hold uses, so a manual smart zoom lets go the way an automatic one does. Tests: `a_chain_of_keys_ends_one_hold_after_the_last`, `a_gap_longer_than_the_hold_breaks_the_chain`, `no_key_near_the_start_leaves_the_zoom_alone`, `the_end_is_capped_at_the_clip`.

## typing_on_doc_clock

```rust
pub fn typing_on_doc_clock(paths: &ProjectPaths, doc: &EditDoc) -> Vec<u32>
```

The recording's keystrokes on the doc's clock, sorted: `typing.json` (event time) shifted onto the clip clock the way the seed shifts every region (`seed::output_shift`), then through the doc's own cuts and speed spans (`TimeMap::build(trim, cuts, speed, clip_ms)` then `out_of`) onto the output clock every zoom is on.

## refit

```rust
pub fn refit(doc: &mut EditDoc, id: &str, paths: &ProjectPaths)
```

Refit zoom `id`'s end to the typing after its start if it is a smart-typing zoom and the recording has typing there; otherwise the zoom is left exactly as the op set it. The cap is `region::dur_bound(doc)`, or `clip_ms` when the doc has no bound yet.

### Used by

- `src-tauri/src/edit/commands.rs` (`apply_edit_op`) - after `api::apply`, for an `UpdateZoom` whose `start_ms` is `Some` or whose `smart_typing` is `Some(true)`.
