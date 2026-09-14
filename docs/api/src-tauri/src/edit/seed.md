# src-tauri/src/edit/seed.rs

Builds the first `EditDoc` from raw recording data so the editor opens with the same visual result as today's exporter, then writes it as `edit.json`. On subsequent opens `load_or_seed` returns the existing file, migrated to `DOC_VERSION` if needed (schema migrations + `clip_ms` backfill live in `edit/migrate.rs` - see `docs/api/src-tauri/src/edit/migrate.md` - and are re-exported here as `seed::true_duration_ms` and `seed::output_shift`, the latter `pub(crate)` since its only external caller, `ai::commands::build_plan`, is in-crate). The three public helpers (`zooms_from_regions`, `layout_from_actions`, `actions_on_output_clock`) are pure converters and are independently unit-tested. `build_default` (now `pub(crate)`) also seeds recorded spotlight holds as editable Spotlight effect regions (via `spotlight_effects` + `export::hold::hold_spans`), so a hotkey-held spotlight appears as an editable/removable timeline pill instead of being baked in, and seeds `clip_ms` to the same true clip duration as `trim.out_ms`.

**`load_or_seed` itself, and the locking around it, live in `edit/seed_lock.rs`** (split out - `seed.rs` was at the 200-line budget), re-exported here (`pub use crate::edit::seed_lock::load_or_seed;`) so callers keep using the `seed::` path. See `docs/api/src-tauri/src/edit/seed_lock.md` for the full self-locking / two-phase-locking design (bug-sweep-2 Task 7 round 2) and `docs/api/src-tauri/src/edit/lock.md` for the shared per-folder lock it's built on.

### One clock: every seeded region is output time

Everything the recording hands the seed - mouse events, the action log, auto/manual zoom regions - is on the EVENT clock (`ev_t`, relative to when the input trackers started). Every region list in an `EditDoc` (`zooms`, `effects`, `layout`, `camera_moves`) is on the OUTPUT clock (`out_t`, 0 = first video frame), because that is the clock the editor timeline, the TS preview and the exporter's `FramePose::out_t` all use. `build_default` therefore computes `shift = events_ms - video_start` once (both from `build_timeline`; real recordings sit around -800 ms) and converts before storing:

- zoom regions: the `shift` is applied to each region's `start_ms`/`end_ms`, saturating at 0.
- layout + effects: the ACTION LOG is shifted instead (`actions_on_output_clock`), then `layout_from_actions`/`spotlight_effects` derive spans from it against the output-time `dur_ms`. Shifting the source rather than the derived spans is what keeps the layout segments tiling `[0, dur_ms]` with no gap at the tail.

Clip duration and the `video_start`/`events_ms` base both come from `build_timeline`.

## zooms_from_regions

```rust
pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom>
```

Converts a slice of auto/manual `ZoomRegion`s into `Zoom` entries with stable ids. Pure - identical input always yields identical output.

### Inputs

- `regions: &[ZoomRegion]` - raw zoom regions emitted by `autozoom::generate` or `manual::from_actions`, in source order. *Why the raw regions rather than an intermediate struct:* the seed must match the exporter's inputs exactly so a seeded render is byte-identical to today's export; reusing the same region type keeps a single source of truth.*

### Returns

`Vec<Zoom>` of the same length as `regions`. Ids are `z0`, `z1`, ... in input order. Every zoom uses `ZoomTarget::Cursor` - it follows the cursor; the region's click anchor is not carried.

### Implementation

1. Enumerate `regions`.
2. For each `(i, r)`: assign `id = "z{i}"`, copy `start_ms`, `end_ms`, `scale` from the region, convert `easing` via `easing_str`, set `target = ZoomTarget::Cursor`.
3. *Why `Cursor` (2026-09-14; it was `Fixed { anchor }` before):* an auto zoom fires on a click, and at that instant the cursor IS the region's anchor, so a following zoom lands where the pinned one did and then tracks the hand instead of staying nailed to the click point. The owner found a fresh recording's auto zooms showing as "Region" in the inspector and ruled they should follow. A user who wants a pinned aim switches that zoom to Region. The old rationale - a seeded doc rendering byte-identically to the pre-editor export - is no longer a goal; `regions_from_doc` still round-trips every other field (`fromedit_tests::regions_round_trip_through_edit_doc`).

### Behaviors

- `ids_are_deterministic_and_count_matches` - three regions yield `z0`, `z1`, `z2` with `len == 3`.
- `empty_regions_make_no_zooms` - empty input produces an empty vec.
- `fields_map_faithfully_and_the_zoom_follows_the_cursor` - all fields copy correctly; the target is `Cursor`.
- `easing_names_map` - `Smooth -> "smooth"`, `Linear -> "linear"`, `Spring -> "spring"`.
- `easing_str` also renders `Easing::Cubic` back to its `cubic(x1,y1,x2,y2)` wire form via `export::cubic::format_cubic`, so a seeded doc round-trips a custom curve byte-identically (unlike `Spring`, whose params a string cannot hold).

## layout_from_actions

```rust
pub fn layout_from_actions(actions: &[ActionEvent], dur_ms: u32) -> Vec<LayoutSeg>
```

Converts the `SetLayout` action track into consecutive, non-overlapping `LayoutSeg` entries that cover `[0, dur_ms]`. Pure.

### Inputs

- `actions: &[ActionEvent]` - the full action log; only `ActionKind::SetLayout` events are extracted. *Why the full log rather than pre-filtered:* avoids a second allocation in the caller; the filter is a single-pass `if let`.*
- `dur_ms: u32` - clip duration used as the `end_ms` of the last segment. *Why supplied separately:* the action log has no concept of clip duration; that comes from the frame timeline.*

Both arguments share ONE clock. The seed passes both on the output clock (actions via `actions_on_output_clock`, `dur_ms` from `build_timeline`), so the segments it stores are output-time like every other `EditDoc` region.

### Returns

`Vec<LayoutSeg>` where each segment's `end_ms` equals the next segment's `start_ms`, and the final segment's `end_ms = dur_ms`. An empty action log produces a single `"screen"` segment from `0` to `dur_ms` (mirroring `LayoutTrack`'s initial state).

### Implementation

1. Seed `switches` with `(0, LayoutId::Screen)` to guarantee a baseline segment at the start.
2. Walk `actions`; on each `SetLayout(id)` push `(a.t, id)`.
3. Iterate `switches` with lookahead: `end = switches[i+1].start` or `dur_ms` for the last. Apply `max(start)` so zero-duration back-to-back switches cannot produce a negative span.
4. Convert each switch to `LayoutSeg { id: "l{i}", ..., layout: layout_name(id) }`.
5. `layout_name` uses `serde_json::to_value` to extract the wire string - ensures the segment's layout string always matches how `LayoutId` serializes to JSON, so no separate lookup table can drift.

### Behaviors

- `empty_layout_track_is_one_screen_span` - no actions -> one `"screen"` segment `[0, dur_ms]` with id `"l0"`.
- `layout_switches_make_consecutive_spans_to_duration` - two switches at 1000 ms and 3000 ms into an 8000 ms clip produce three consecutive segments with correct ids and layout names.

`load_or_seed` (`pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc`) is `pub use`-re-exported from `edit::seed_lock` here, the same way `true_duration_ms`/`output_shift` are re-exported from `edit::migrate` below - so existing callers keep using the `seed::` path. It is DEFINED, and fully documented (self-locking behavior, the fast-path/slow-path split, why `edit::commands::apply_edit_op` calls the lower-level `seed_lock::load_or_seed_locked` instead, its full caller list, and its `Behaviors`), in `docs/api/src-tauri/src/edit/seed_lock.md`.

## actions_on_output_clock

```rust
pub fn actions_on_output_clock(actions: &[ActionEvent], shift: i64) -> Vec<ActionEvent>
```

PURE: the recorded action log with every timestamp moved onto the output clock by `shift` (`events_ms - video_start`), saturating at 0; `kind` is untouched. Both region seeders (`layout_from_actions`, `spotlight_effects`) and the renderer's recorded-layout fallback (`EditState::load`, used when `doc.layout` is empty) build from the SAME shifted log, so a seeded track and a fallback track can never end up on different clocks.

### Behaviors

- `action_timestamps_shift_onto_the_output_clock_and_saturate_at_zero` - `t: 2000` with `shift: -800` becomes `1200`; `t: 100` clamps to `0` instead of wrapping; kinds are preserved.

### Used by

- `src-tauri/src/edit/seed.rs` - `build_default`, before deriving layout segments and spotlight regions
- `src-tauri/src/export/render/render_edit.rs` - `EditState::load`'s recorded-action layout fallback

`true_duration_ms` and `output_shift` are re-exported here (`use crate::edit::migrate::{true_duration_ms, output_shift};`) so existing callers keep using the `seed::` path; their migration/clock siblings live in `docs/api/src-tauri/src/edit/migrate.md`.
