# src-tauri/src/edit/migrate.rs

Doc-version migrations plus the event/output clock shift they need, split out of `seed.rs` (which was at the 200-line budget). `seed::load_or_seed` calls `migrate` right after load/build, before `effects::lift_always_on_spotlight`. `true_duration_ms` is re-exported as `seed::true_duration_ms` so existing external callers are unaffected by the split.

## migrate

```rust
pub(crate) fn migrate(doc: &mut EditDoc, paths: &ProjectPaths) -> bool
```

Brings a loaded doc up to `DOC_VERSION` AND backfills `EditDoc::clip_ms`, returning whether either changed so `load_or_seed` knows to persist it. One `if doc.version < N` step per schema bump, applied in order, so a future version adds a step without touching the earlier ones. The version is ALWAYS stamped to `DOC_VERSION` at the end of its step - including when the step could not do its work - so a doc is never migrated twice.

The `clip_ms` backfill is a SEPARATE step, not gated by the version check: a doc already at `DOC_VERSION` but written before `clip_ms` existed also reads it as `0` (its `#[serde(default)]`), so the backfill runs whenever `doc.clip_ms == 0`, independent of `version`. It only writes when `true_duration_ms` resolves to something nonzero, so a doc with no timeline doesn't get spuriously marked "changed" every load.

### v1 -> v2 (`v1_to_v2`)

v1 seeded `effects` and `layout` straight off the recorded action log (event clock) while its zooms were already shifted to output time, so on a real recording those regions sat ~`video_start - events_ms` (≈800 ms) early: a spotlight fired before its pill in the editor, and the export disagreed with the preview. `v1_to_v2` adds `output_shift(paths)` to every `effects`/`layout` `start_ms`/`end_ms`, saturating at 0. `zooms` and `camera_moves` are left alone - both were already output-time.

`output_shift` returns `events_ms - video_start` from `build_timeline`, or `0` when `events.json` cannot be loaded. A missing timeline makes the offset unknowable, so the regions are left exactly where they are - but the doc is still stamped v2, because re-running the migration later (when a timeline WOULD resolve) would shift regions the user has since edited in output time. Re-exported as `seed::output_shift`; `ai::commands::build_plan` reuses this SAME recipe to put the AI director transcript (`ai::backend::timeline::serialize`) on the output clock, rather than re-deriving the shift.

### `output_shift` used by

- `src-tauri/src/edit/migrate.rs` - `v1_to_v2`
- `src-tauri/src/ai/commands.rs` - `build_plan`, via the `seed::output_shift` re-export

*Known limit:* a doc seeded before zooms themselves were shifted to output time is also `version: 1` and its zooms are NOT corrected - the version was not bumped at the time, so the two cases are indistinguishable on disk.

### Behaviors

- `v1_docs_migrate_their_regions_once` (in `seed_tests.rs`) - a v1 doc's effects/layout shift by `events_ms - video_start` (saturating at 0) and come back as v2, with `clip_ms` backfilled too; a second `load_or_seed` leaves them untouched.
- `a_v1_doc_without_a_timeline_becomes_v2_unshifted` (in `seed_tests.rs`) - no `events.json`/`sync.json` -> regions AND `clip_ms` unchanged, version still bumped.
- `clip_ms_backfills_on_an_already_current_version_doc_too` (in `seed_tests.rs`) - a doc already at `DOC_VERSION` but missing `clip_ms` still gets it backfilled.

## true_duration_ms

```rust
pub fn true_duration_ms(paths: &ProjectPaths) -> u32
```

The recording's TRUE full duration (ms) - `video_end - video_start` from the real capture timeline (`build_timeline`), independent of any user `trim.out_ms` selection. Preview helpers that need "how long is the whole clip" call this rather than reading `trim.out_ms`, which - once a user actually trims - no longer means the recording's length (it means the trim-out point). Also what `migrate` backfills `EditDoc::clip_ms` from. Returns `0` if `events.json` cannot be loaded.

### Used by

- `src-tauri/src/export/preview/preview_track.rs` - `ensure_proxy`'s re-time stretch factor
- `src-tauri/src/export/preview/thumbs.rs` - `ensure_thumbs`'s filmstrip spacing
- `src-tauri/src/edit/migrate.rs` - `migrate`'s `clip_ms` backfill
- `src-tauri/src/ai/commands.rs` - `build_plan`'s clip-length fallback
