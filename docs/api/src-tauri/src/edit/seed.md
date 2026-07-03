# src-tauri/src/edit/seed.rs

Builds the first `EditDoc` from raw recording data so the editor opens with the same visual result as today's exporter, then writes it as `edit.json`. On subsequent opens `load_or_seed` returns the existing file untouched. The two public helpers (`zooms_from_regions`, `layout_from_actions`) are pure converters and are independently unit-tested. `build_default` also seeds recorded spotlight holds as editable Spotlight effect regions (via `spotlight_effects` + `export::hold::hold_spans`), so a hotkey-held spotlight appears as an editable/removable timeline pill instead of being baked in. Auto/manual zoom regions come out in event time (click/press timestamps), but the editor timeline is output time (0 = first video frame), so `build_default` shifts every seeded zoom by `events_ms - video_start` (the same `out = et + events_ms - video_start` mapping `click_track` uses) - so a seeded zoom's pill sits where the zoom fires, and edited/added zooms (already output-time) stay consistent. Clip duration and that `video_start`/`events_ms` base both come from `build_timeline`.

## zooms_from_regions

```rust
pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom>
```

Converts a slice of auto/manual `ZoomRegion`s into `Zoom` entries with stable ids. Pure - identical input always yields identical output.

### Inputs

- `regions: &[ZoomRegion]` - raw zoom regions emitted by `autozoom::generate` or `manual::from_actions`, in source order. *Why the raw regions rather than an intermediate struct:* the seed must match the exporter's inputs exactly so a seeded render is byte-identical to today's export; reusing the same region type keeps a single source of truth.*

### Returns

`Vec<Zoom>` of the same length as `regions`. Ids are `z0`, `z1`, ... in input order. Each zoom uses `ZoomTarget::Fixed { x: anchor.x, y: anchor.y }` (not `Cursor`), preserving the click-site anchor from the recording.

### Implementation

1. Enumerate `regions`.
2. For each `(i, r)`: assign `id = "z{i}"`, copy `start_ms`, `end_ms`, `scale` from the region, convert `easing` via `easing_str`, set `target = ZoomTarget::Fixed { x: r.anchor.x as f32, y: r.anchor.y as f32 }`.
3. *Why `Fixed` not `Cursor`:* the anchor is the click point that triggered the zoom - locking it to a fixed coordinate keeps the camera in the same spot a re-render of the same recording would use, avoiding drift from cursor position at playback time.

### Behaviors

- `ids_are_deterministic_and_count_matches` - three regions yield `z0`, `z1`, `z2` with `len == 3`.
- `empty_regions_make_no_zooms` - empty input produces an empty vec.
- `fields_map_faithfully_and_anchor_is_preserved_as_fixed` - all fields copy correctly; target is `Fixed` with the exact anchor coordinates.
- `easing_names_map` - `Smooth -> "smooth"`, `Linear -> "linear"`, `Spring -> "spring"`.

## layout_from_actions

```rust
pub fn layout_from_actions(actions: &[ActionEvent], dur_ms: u32) -> Vec<LayoutSeg>
```

Converts the `SetLayout` action track into consecutive, non-overlapping `LayoutSeg` entries that cover `[0, dur_ms]`. Pure.

### Inputs

- `actions: &[ActionEvent]` - the full action log; only `ActionKind::SetLayout` events are extracted. *Why the full log rather than pre-filtered:* avoids a second allocation in the caller; the filter is a single-pass `if let`.*
- `dur_ms: u32` - clip duration used as the `end_ms` of the last segment. *Why supplied separately:* the action log has no concept of clip duration; that comes from the frame timeline.*

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

## load_or_seed

```rust
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc
```

Returns the existing `EditDoc` for a project, or builds and persists a default one if none exists. Either way it then runs `effects::lift_always_on_spotlight` so an always-on `clickfx.spotlight` becomes an editable full-span timeline region (a lightweight migration that also upgrades previously-saved docs), persisting the doc when that or a fresh seed changed it.

### Inputs

- `paths: &ProjectPaths` - resolved paths for the project directory. *Why `ProjectPaths` rather than individual file paths:* a single struct prevents callers from assembling mismatched paths.*

### Returns

`EditDoc` - always succeeds. On a new project the returned doc is identical to what `build_default` would produce for a fresh export, so the editor opens in a consistent state.

### Implementation

1. Load `EditDoc::load(paths.edit())` if present (`fresh = false`), else `build_default(paths)` (`fresh = true`).
2. Run `effects::lift_always_on_spotlight(&mut doc)` - converts an always-on spotlight toggle into an editable full-span region (idempotent; returns whether it changed the doc).
3. If the doc was freshly seeded OR the lift changed it, attempt `doc.save(paths.edit())` (failure silently ignored - the caller still gets a valid doc).
4. Return the doc.

### Used by

- `src-tauri/src/edit/commands.rs` - `get_edit` and `apply_edit_op` both call this as their first step
- `src-tauri/src/ai/commands.rs` - loads the doc before applying an AI plan
