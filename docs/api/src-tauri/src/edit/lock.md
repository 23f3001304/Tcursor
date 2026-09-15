# src-tauri/src/edit/lock.rs

The single per-project-folder lock registry guarding every write `edit.json` can receive, wherever it originates - split into its own file (bug-sweep-2 Task 7 round 2) so `edit::commands` and `edit::seed` share exactly ONE lock instead of each rolling its own (round 1 of this fix had `edit::commands` roll its own, which left `edit::seed::load_or_seed`'s own internal write unlocked everywhere else it's called from - see below).

## doc_lock

```rust
pub(crate) fn doc_lock(p: &ProjectPaths) -> Arc<Mutex<()>>
```

One lock per project folder (H3). Without it, any two of `edit::commands::apply_edit_op`'s read-modify-write, `save_edit`'s blind write, and `edit::seed::load_or_seed`'s OWN internal seed/migrate/lift write have no ordering guarantee against each other - whichever reaches disk LAST wins even if it read a now-stale copy, silently discarding whatever another writer just wrote (`EditDoc::save`'s tmp+rename is atomic w.r.t. TORN files, but says nothing about WHICH document wins a race).

### Inputs

- `p: &ProjectPaths` - the project whose lock to return.

### Returns

`Arc<Mutex<()>>` - a clone of the shared per-folder lock, lazily created on first request and kept in a process-global `Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>` behind a `OnceLock`. Cloning the `Arc` (rather than returning a guard while holding the map's own lock) lets the caller lock/unlock the per-folder mutex without contending on the map lock for the duration of its own work.

### Implementation

1. `LOCKS.get_or_init(...)` - lazily initialize the process-global registry.
2. Lock the registry, `.entry(p.folder.clone()).or_insert_with(|| Arc::new(Mutex::new(())))`, `.clone()` the `Arc` out, drop the registry lock (end of statement).

### Round 2 fix: why `load_or_seed` itself needs this lock, not just the three commands

`load_or_seed` (`edit::seed`) is the ONE place every caller in the tree reaches `edit.json` through - not just `edit::commands`' three IPC entry points. Round 1 of this fix only locked those three, leaving `load_or_seed`'s own seed/migrate/lift write unlocked everywhere else it's called from:

- `src-tauri/src/ai/run.rs` (`propose`)
- `src-tauri/src/export/preview/session.rs` (`reuse`, `build` - the warm-cache build/reuse)
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`)
- `src-tauri/src/export/render/render_edit.rs` (`EditState::load`)
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`)
- `src-tauri/src/export/preview/preprocess.rs` (`run`)
- `src-tauri/src/export/preview/thumbs.rs` (`preview_audio_shifts`)

During the editor-mount window (5-6 concurrent preview IPC calls firing on a not-yet-seeded/migrated doc), an unlocked caller could read the doc before a locked `apply_edit_op` wrote, then land its own stale seed/migrate/lift write after - the same lost-update shape H3 already described for the three commands, just reachable from more places. Taking this lock INSIDE `load_or_seed` itself (`edit::seed::load_or_seed`, see `seed.md`) is what actually covers all of them; `edit::commands`' three commands still take it too, for the parts of their own work `load_or_seed` doesn't cover (`apply_edit_op`'s `apply` + save, `save_edit`'s save). `apply_edit_op` takes it ONCE across both `load_or_seed`'s locked core (`seed::load_or_seed_locked`) and its own write, never twice - `std::sync::Mutex` is NOT reentrant, so calling the self-locking `load_or_seed` while already holding this lock would deadlock.

### Lock ordering vs. `export::preview::session::WarmSlot`

`WarmSlot::with` holds its `gate` mutex across the whole `reuse`/`make`/`work` call (never its `cell` mutex, which is only ever held to swap the cached entry in/out - see `session.md`). `reuse`/`build` call `load_or_seed` while `gate` is held, so the only ordering this lock ever participates in is `gate -> doc_lock`.

The reverse (`doc_lock -> gate`) never occurs: nothing reachable from inside this lock's critical section (`edit::seed`, `edit::migrate`, `edit::ops::effects`, `EditDoc::save`) ever touches `WarmSlot`, `PreviewSession`, or `process::proc::generate_once`'s lock - confirmed by grep, none of those types/functions are referenced anywhere under `edit::`. `generate_once`'s callers (`thumbs.rs`, `preview_track.rs`) all call `load_or_seed`-touching helpers (`load_or_seed` itself, `preview_audio_shifts`) BEFORE taking `generate_once`'s lock, never inside its closure, so there is no `generate_once -> doc_lock` ordering either.

### Behaviors

- `doc_lock_is_shared_per_folder_and_distinct_across_folders` - two calls for the same folder return the same `Arc` (`Arc::ptr_eq`); different folders return different ones.

### Used by

- `src-tauri/src/edit/commands.rs` - `apply_edit_op`, `save_edit`
- `src-tauri/src/edit/seed.rs` - `load_or_seed` (self-locking)
