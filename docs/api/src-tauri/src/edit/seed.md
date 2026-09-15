# src-tauri/src/edit/seed.rs

Builds the first `EditDoc` from raw recording data so the editor opens with the same visual result as today's exporter, then writes it as `edit.json`. On subsequent opens `load_or_seed` returns the existing file, migrated to `DOC_VERSION` if needed (schema migrations + `clip_ms` backfill live in `edit/migrate.rs` - see `docs/api/src-tauri/src/edit/migrate.md` - and are re-exported here as `seed::true_duration_ms` and `seed::output_shift`, the latter `pub(crate)` since its only external caller, `ai::run::propose`, is in-crate). The three public helpers (`zooms_from_regions`, `layout_from_actions`, `actions_on_output_clock`) are pure converters and are independently unit-tested. `build_default` (now `pub(crate)`) also seeds recorded spotlight holds as editable Spotlight effect regions (via `spotlight_effects` + `export::hold::hold_spans`), so a hotkey-held spotlight appears as an editable/removable timeline pill instead of being baked in, and seeds `clip_ms` to the same true clip duration as `trim.out_ms`.

**`load_or_seed` and the locking around it also live here** (`load_or_seed`, `load_or_seed_locked`, `derive_seed_inputs`, `needs_seed_write`, documented below), with their tests in the sibling `seed_lock_tests.rs`. Bug-sweep-2 Task 7 round 2 (H3): a code-review pass on round 1 found that locking only `edit::commands`' three IPC entry points left `load_or_seed`'s own internal seed/migrate/lift write unlocked everywhere else it is called from (~8 other call sites across `ai`/`export` - see `docs/api/src-tauri/src/edit/lock.md` for the full list and the lock-ordering analysis vs. `WarmSlot::gate`).

### One clock: every seeded region is output time

Everything the recording hands the seed - mouse events, the action log, auto/manual zoom regions - is on the EVENT clock (`ev_t`, relative to when the input trackers started). Every region list in an `EditDoc` (`zooms`, `effects`, `layout`, `camera_moves`) is on the OUTPUT clock (`out_t`, 0 = first video frame), because that is the clock the editor timeline, the TS preview and the exporter's `FramePose::out_t` all use. `build_default` therefore computes `shift = events_ms - video_start` once (both from `build_timeline`; real recordings sit around -800 ms) and converts before storing:

- zoom regions: the `shift` is applied to each region's `start_ms`/`end_ms`, saturating at 0.
- layout + effects: the ACTION LOG is shifted instead (`actions_on_output_clock`), then `layout_from_actions`/`spotlight_effects` derive spans from it against the output-time `dur_ms`. Shifting the source rather than the derived spans is what keeps the layout segments tiling `[0, dur_ms]` with no gap at the tail.

Clip duration and the `video_start`/`events_ms` base both come from `build_timeline`.

## easing_str

```rust
fn easing_str(e: Easing) -> String
```

The wire-name for an `Easing`, and the exact inverse of `easing_from` (`export/render/fromedit.md`). The six named curves emit their bare word; `Spring`, `Cubic` and `Keys` carry their whole curve, so they emit their own parameterised form through `spring::format_spring` / `cubic::format_cubic` / `keys::format_keys` instead. *Why it matters that this is exact:* a seeded doc is re-read by the exporter through `easing_from`, so any variant that could not write itself back would silently render as the config's default on the next open. `a_keys_curve_round_trips_through_the_wire` pins that for `Keys`, which is the widest payload of the three.

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
2. For each `(i, r)`: assign `id = "z{i}"`, copy `start_ms`, `end_ms`, `scale` from the region, convert `easing` via `easing_str`, set `target = ZoomTarget::Cursor`. `easing_out` (M3) is stored only when `r.easing_out != r.easing` - so a generated region, whose two ramps always agree, writes exactly the JSON it wrote before the field existed, while the round trip through `regions_from_doc` still carries a split one losslessly.
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

## load_or_seed

```rust
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc
```

Return an existing `edit.json`, else build+migrate+lift one and persist it. Two phases:

1. **FAST PATH** (no lock): an unlocked read that turns out already fully current (`!needs_seed_write`) returns immediately - the common case once a project has been opened once. Nothing to write, so nothing to race, so no lock is ever taken.
2. **SLOW PATH**: precompute every ffprobe-backed input (`derive_seed_inputs`) BEFORE taking `edit::lock::doc_lock`, then hand off to `load_or_seed_locked` for the actual re-read-and-maybe-write under the lock. The lock is therefore only ever held across cheap in-memory work plus one atomic save - never across a `build_timeline`/ffprobe call.

### Inputs

- `paths: &ProjectPaths` - resolved paths for the project directory.

### Returns

`EditDoc` - always succeeds. On a new project the returned doc is identical to what `build_default` would produce for a fresh export, so the editor opens in a consistent state.

### Self-locking - who should (and shouldn't) call this

The right choice for the ~8 callers across the tree that just want a coherent doc and make no `edit.json` write of their own right after (`ai::run::propose`, `export::preview::session`'s warm-cache build/reuse, `export::render::*`, `export::cursor::cursorpreview`, `export::preview::{preprocess,thumbs}`, and this crate's own `edit::commands::get_edit`).

`edit::commands::apply_edit_op` is the ONE caller that DOES need to make its own write (the op + save) immediately after, under the SAME lock - it calls `load_or_seed_locked` directly instead of this function. `std::sync::Mutex` is NOT reentrant, so calling this self-locking function while already holding `edit::lock::doc_lock` would deadlock.

### Behaviors

(Pinned in `seed_tests.rs`)

- `v1_docs_migrate_their_regions_once`, `a_v1_doc_without_a_timeline_becomes_v2_unshifted`, `clip_ms_backfills_on_an_already_current_version_doc_too`.

## load_or_seed_locked

```rust
pub(crate) fn load_or_seed_locked(paths: &ProjectPaths, precomputed_default: Option<EditDoc>, shift: i64, true_dur: u32) -> EditDoc
```

The locked core of `load_or_seed`.

### Inputs

- `paths: &ProjectPaths` - resolved paths for the project directory.
- `precomputed_default: Option<EditDoc>` / `shift: i64` / `true_dur: u32` - from `derive_seed_inputs`, computed either inside `load_or_seed`'s slow path or by `edit::commands::apply_edit_op` before it takes the lock.

### Precondition

**Caller MUST already hold `edit::lock::doc_lock(paths)`** - this function takes no lock of its own, so `apply_edit_op` can call it as part of one larger locked critical section that also covers its own `apply` + save.

### Implementation

1. Re-read `edit.json` fresh via `EditDoc::load` - authoritative, since a concurrent writer may have changed it since any earlier unlocked snapshot (the one `derive_seed_inputs` saw), and that write must win. `None` (missing/corrupt) falls back to `precomputed_default`, or builds fresh via `build_default(paths)` if that wasn't precomputed either (the rare corrupt-file edge case - see `derive_seed_inputs`).
2. `migrate(&mut doc, shift, true_dur)` - schema upgrades AND the `clip_ms` backfill, using the caller-supplied `shift`/`true_dur` rather than re-deriving them (keeps this function's own body ffprobe-free).
3. `effects::lift_always_on_spotlight(&mut doc)` - converts an always-on spotlight toggle into an editable full-span region.
4. If the doc was freshly built OR the migration OR the lift changed it, `doc.save(&paths.edit())` (failure silently ignored - the caller still gets a valid doc).
5. Return the doc.

### Why reusing `precomputed_default`/`shift`/`true_dur` here is safe even though they may predate this lock

They depend only on the recording's STATIC artifact files, never on `edit.json`'s own content - see `derive_seed_inputs` below for the monotonicity argument for why a value it decided NOT to compute can never turn out to have been needed after all.

### Behaviors

- `locked_reread_wins_over_a_write_that_lands_between_derive_and_lock` (`seed_lock_tests.rs`) - the round-2 review's core ask: a v1 doc is seeded, `derive_seed_inputs` takes its unlocked snapshot, then a CONCURRENT write (simulating any of the ~8 unlocked callers racing an `apply_edit_op`) applies a real zoom and saves; `load_or_seed_locked` (using the now-stale precomputed inputs) must still see and preserve that zoom, both in its return value and in what lands on disk.

### Used by

- `load_or_seed` in this file (after taking the lock)
- `src-tauri/src/edit/commands.rs` - `apply_edit_op` (under its own lock acquisition, which also covers the op + save)

## derive_seed_inputs

```rust
pub(crate) fn derive_seed_inputs(paths: &ProjectPaths, unlocked: &Option<EditDoc>) -> (Option<EditDoc>, i64, u32)
```

Precomputes the (possibly ffprobe-backed) inputs `load_or_seed_locked` might need, from an UNLOCKED snapshot of the doc.

### Inputs

- `paths: &ProjectPaths` - resolved paths for the project directory.
- `unlocked: &Option<EditDoc>` - an UNLOCKED read of `edit.json` (`None` if missing/corrupt), used only to DECIDE what to precompute, never as the actual base for mutation (that's always the fresh re-read inside `load_or_seed_locked`).

### Returns

`(Option<EditDoc>, i64, u32)` - `precomputed_default` (a full fresh-built doc, `Some` only when `unlocked` was `None`), `shift` (for `migrate`'s v1->v2 step, `0` when not needed), `true_dur` (for `migrate`'s `clip_ms` backfill, `0` when not needed).

### Why this is safe to run before taking `edit::lock::doc_lock`

None of the three values read `edit.json`: `build_default` only reads the recording's OTHER artifact files (`settings.json`/`events.json`/`actions.json`/`typing.json`), and `output_shift`/`true_duration_ms` only read `events.json`/`sync.json`.

### Why the gating never misses a value that turns out to be needed (monotonicity)

`doc.version` only ever increases (nothing decrements it) and `doc.clip_ms` only ever moves from `0` to nonzero (nothing resets it) - so if the unlocked snapshot already had `version >= 2` / `clip_ms != 0`, the freshly re-read doc under the lock is GUARANTEED to as well; a gated-off value is only ever stale-and-UNUSED, never stale-and-insufficient. The one theoretical exception is a doc that was corrupt at the unlocked read (which this function sees as `None`, same as missing) and got fixed by something else before the lock - not reachable through this app's own write paths. Even then, `migrate` treats `true_dur == 0` as its normal "unknowable, don't guess" case (bails cleanly, matching `true_duration_ms`'s own contract), so the affected backfill is simply deferred to a later `load_or_seed` call rather than wrong or lost - the chosen "bail", not "loop", resolution for a re-check that (in practice) never actually invalidates the precomputed inputs.

### Implementation

1. `needs_shift = unlocked.version < 2` (or `false` if `unlocked` is `None` - a fresh doc is built already-current, so `migrate` will no-op regardless).
2. `needs_true_dur = unlocked.clip_ms == 0` (or `false` if `None`, same reasoning).
3. `precomputed_default = build_default(paths)` when `unlocked.is_none()`, else `None`.
4. `shift = output_shift(paths)` when `needs_shift`, else `0`.
5. `true_dur = true_duration_ms(paths)` when `needs_true_dur`, else `0`.

### Behaviors

- `derive_seed_inputs_skips_ffprobe_work_for_an_already_current_doc` - a doc already at `DOC_VERSION` with `clip_ms` set precomputes nothing (`precomputed_default: None`, `shift: 0`, `true_dur: 0`) - the pure "flip the Spotlight toggle" write is the most common non-fresh case `load_or_seed` handles, and it must never pay for a `build_timeline` pass.

### Used by

- `load_or_seed`'s slow path in this file
- `src-tauri/src/edit/commands.rs` - `apply_edit_op`, before taking the lock

## needs_seed_write

```rust
fn needs_seed_write(doc: &EditDoc) -> bool
```

Whether `load_or_seed` would need to write `doc` back to disk: an old-schema doc, one still missing the `clip_ms` backfill, or one with the always-on spotlight toggle on but no lifted region yet. Exactly mirrors the three conditions `migrate`/`lift_always_on_spotlight` check - a doc this returns `false` for is genuinely a no-op read, so `load_or_seed`'s FAST PATH skips the per-folder lock entirely for it.

### Behaviors

- `needs_seed_write_is_false_for_a_fully_current_doc`
- `needs_seed_write_is_true_for_an_old_version`
- `needs_seed_write_is_true_when_clip_ms_is_unknown`
- `needs_seed_write_is_true_when_the_spotlight_toggle_needs_lifting`

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
