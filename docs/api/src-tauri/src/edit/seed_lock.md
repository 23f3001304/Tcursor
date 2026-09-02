# src-tauri/src/edit/seed_lock.rs

The locked write path behind `seed::load_or_seed` - split out of `seed.rs` (which was at the 200-line budget) so the two-phase locking shape (fast path / precompute-then-lock) has room to be documented properly. Bug-sweep-2 Task 7 round 2 (H3): a code-review pass on round 1 found that locking only `edit::commands`' three IPC entry points left `seed::load_or_seed`'s own internal seed/migrate/lift write unlocked everywhere else it's called from (~8 other call sites across `ai`/`export` - see `docs/api/src-tauri/src/edit/lock.md` for the full list and the lock-ordering analysis vs. `WarmSlot::gate`).

## load_or_seed

```rust
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc
```

Return an existing `edit.json`, else build+migrate+lift one and persist it. Re-exported as `seed::load_or_seed` so every existing caller keeps using that path. Two phases:

1. **FAST PATH** (no lock): an unlocked read that turns out already fully current (`!needs_seed_write`) returns immediately - the common case once a project has been opened once. Nothing to write, so nothing to race, so no lock is ever taken.
2. **SLOW PATH**: precompute every ffprobe-backed input (`derive_seed_inputs`) BEFORE taking `edit::lock::doc_lock`, then hand off to `load_or_seed_locked` for the actual re-read-and-maybe-write under the lock. The lock is therefore only ever held across cheap in-memory work plus one atomic save - never across a `build_timeline`/ffprobe call.

### Inputs

- `paths: &ProjectPaths` - resolved paths for the project directory.

### Returns

`EditDoc` - always succeeds. On a new project the returned doc is identical to what `build_default` would produce for a fresh export, so the editor opens in a consistent state.

### Self-locking - who should (and shouldn't) call this

The right choice for the ~8 callers across the tree that just want a coherent doc and make no `edit.json` write of their own right after (`ai::commands::build_plan`, `export::preview::session`'s warm-cache build/reuse, `export::render::*`, `export::cursor::cursorpreview`, `export::preview::{preprocess,thumbs}`, and this crate's own `edit::commands::get_edit`).

`edit::commands::apply_edit_op` is the ONE caller that DOES need to make its own write (the op + save) immediately after, under the SAME lock - it calls `load_or_seed_locked` directly instead of this function. `std::sync::Mutex` is NOT reentrant, so calling this self-locking function while already holding `edit::lock::doc_lock` would deadlock.

### Behaviors

(Pinned via `seed::load_or_seed`'s re-export in `seed_tests.rs` - unchanged by this refactor)

- `v1_docs_migrate_their_regions_once`, `a_v1_doc_without_a_timeline_becomes_v2_unshifted`, `clip_ms_backfills_on_an_already_current_version_doc_too` - see `seed.md`.

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

They depend only on the recording's STATIC artifact files, never on `edit.json`'s own content - see `derive_seed_inputs`'s doc comment for the monotonicity argument for why a value it decided NOT to compute can never turn out to have been needed after all.

### Behaviors

- `locked_reread_wins_over_a_write_that_lands_between_derive_and_lock` (`seed_lock_tests.rs`) - the round-2 review's core ask: a v1 doc is seeded, `derive_seed_inputs` takes its unlocked snapshot, then a CONCURRENT write (simulating any of the ~8 unlocked callers racing an `apply_edit_op`) applies a real zoom and saves; `load_or_seed_locked` (using the now-stale precomputed inputs) must still see and preserve that zoom, both in its return value and in what lands on disk.

### Used by

- `src-tauri/src/edit/seed_lock.rs` - `load_or_seed` (after taking the lock)
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

- `src-tauri/src/edit/seed_lock.rs` - `load_or_seed`'s slow path
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
