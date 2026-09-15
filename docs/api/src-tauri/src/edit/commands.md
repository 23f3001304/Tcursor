# src-tauri/src/edit/commands.rs

Tauri IPC command handlers that bridge the frontend editor to the `edit` layer. Three commands cover the full read-mutate-save lifecycle; the frontend never touches `edit.json` directly.

**Locking (H3, bug-sweep-2 Task 7).** All three commands, and `edit::seed::load_or_seed`'s own internal seed/migrate/lift write, serialize through the ONE shared per-folder lock `edit::lock::doc_lock` - see `docs/api/src-tauri/src/edit/lock.md` for why a shared lock is needed at all, the round-2 fix that moved locking INTO `load_or_seed` itself (round 1 only covered these three commands, leaving `load_or_seed`'s ~8 other callers across `ai`/`export` unlocked), and the lock-ordering analysis vs. `export::preview::session::WarmSlot`. This file no longer defines its own `doc_lock` - it was moved to `edit::lock` so `edit::seed` could share it too.

**What the lock does NOT fix:** it only orders the Rust-side I/O; it cannot make a STALE payload fresh. The specific failure it closes off is Rust-level interleaving (e.g. two writers both reading before either writes, which could otherwise discard BOTH pending changes instead of just one). Whether the frontend hands `save_edit` an up-to-date doc in the first place is a TS-side concern - `useEditHistory.swap`'s undo/redo already gets this right by reading `docRef.current` at execution time (inside the shared `enqueue` promise queue) rather than a value closed over when the callback was created; `useDocSettings.write` not yet routing through that same queue is a separate, tracked gap on the TS side.

## get_edit

```rust
#[tauri::command]
pub async fn get_edit(folder: String) -> Result<EditDoc, String>
```

Loads (or seeds) the `EditDoc` for a project folder and returns it to the frontend.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`. On a project that already has `edit.json` this is a small JSON read, but on one that was never preprocessed - a legacy recording, or one whose `preprocess::run` pass failed - `load_or_seed` falls into `seed::build_default`, which gzip-decodes the entire mouse-event log, runs `autozoom::generate` over every sample, and calls `build_timeline`, which spawns two `ffprobe` subprocesses whenever `sync.json` is missing (exactly the legacy case). `useEditorData` calls this on mount as the editor's very first IPC, so as a sync command that seed froze the window on the project-open path - a first-impression surface. `apply_edit_op` and `save_edit` are the same class but far cheaper on the steady state and are left sync for now.

### Inputs

- `folder: String` - absolute path to the project directory. *Why String rather than PathBuf:* Tauri IPC deserializes command arguments from JSON, where paths are strings.

### Returns

`Ok(EditDoc)` on success; `Err(String)` on an unexpected I/O or parse failure (rare - `load_or_seed` silently seeds a default on missing/corrupt files).

### Implementation

Inside `tauri::async_runtime::spawn_blocking`:

1. Construct `ProjectPaths` from `folder`.
2. Delegate FULLY to `edit::seed::load_or_seed`, which returns an existing `edit.json` or builds and writes a default one from the recording. No lock taken here directly - `load_or_seed` is self-locking (`edit::lock::doc_lock`, taken and released internally for the span of any write it makes), and this command does nothing to `edit.json` beyond what `load_or_seed` itself already does. *Why not also wrap this call in its own `doc_lock` acquisition:* the lock is a plain `std::sync::Mutex`, NOT reentrant - a second acquisition here (before calling into a function that acquires it again) would deadlock.
3. Return the doc. A `spawn_blocking` join failure maps to `Err(String)`.

## apply_edit_op

```rust
#[tauri::command]
pub fn apply_edit_op(folder: String, op: EditOp) -> Result<EditDoc, String>
```

Applies one `EditOp` to the project's `EditDoc`, persists the result, and returns the updated doc.

### Inputs

- `folder: String` - project directory. *Why passed on every call:* Tauri commands are stateless; the folder identifies which project to mutate.
- `op: EditOp` - the edit operation to apply. *Why the full enum rather than a free-form JSON blob:* the enum gives serde a compile-time schema, so invalid ops are rejected at deserialization rather than silently ignored mid-mutation.

### Returns

`Ok(EditDoc)` - the post-mutation document, returned so the frontend can update its local state without a second round trip. `Err(String)` if `doc.save` fails (e.g. disk full).

### Implementation

**One lock acquisition covers BOTH the seed/migrate/lift decision AND this op's own apply+save** (round-2 fix) - never two separate acquisitions, since `edit::lock::doc_lock` is not reentrant:

1. Construct `ProjectPaths`.
2. Read `edit.json` UNLOCKED (`EditDoc::load`) purely to decide what `derive_seed_inputs` needs to precompute - this snapshot is never used as the actual mutation base.
3. `edit::seed::derive_seed_inputs(&p, &unlocked)` - precompute any ffprobe-backed inputs `load_or_seed_locked` might need, still unlocked (see `seed.md`).
4. Acquire `edit::lock::doc_lock` for the folder, held for the rest of this function - so nothing else can land between this call's load and save.
5. `edit::seed::load_or_seed_locked(&p, precomputed_default, shift, true_dur)` - re-reads `edit.json` FRESH under the lock (authoritative - not the step-2 snapshot) and applies any still-needed seed/migrate/lift write.
6. Apply `op` via `edit::api::apply` (a clone; the op is inspected once more in the next step).
7. For an `UpdateZoom` whose `start_ms` is `Some` or whose `smart_typing` is `Some(true)`, `ops::smart_zoom::refit` rewrites that zoom's `end_ms` from the recording's typing (see `smart_zoom.md`); every other op leaves every zoom exactly as it set it.
8. Save the mutated doc to `paths.edit()`.
8. Return the mutated doc.

### Behaviors

- `concurrent_apply_edit_op_calls_never_lose_a_write` - 12 threads each call `apply_edit_op` with a distinct `AddZoom` against the SAME project concurrently; all 12 zooms survive on disk (a regression guard against the read-modify-write race the lock exists to close).
- `concurrent_load_or_seed_calls_never_clobber_a_concurrent_apply_edit_op` - round-2 regression: mixes direct `seed::load_or_seed` calls (standing in for the ~8 other unlocked callers round 1 missed) with `apply_edit_op` calls on a v1 doc that also needs migrating, so every `load_or_seed` call takes its own write path; all the concurrently-applied zooms still survive.

## save_edit

```rust
#[tauri::command]
pub fn save_edit(folder: String, doc: EditDoc) -> Result<(), String>
```

Overwrites `edit.json` with a caller-supplied `EditDoc`. Used when the frontend holds a modified doc it has assembled locally (e.g. after a drag operation that batches many changes) rather than applying them one at a time.

### Inputs

- `folder: String` - project directory. *Why the folder rather than a full path:* `ProjectPaths` derives all file locations from a single root, keeping the IPC surface minimal.
- `doc: EditDoc` - the document to persist. *Why accept the whole doc:* allows the frontend to apply multiple local mutations atomically in one save, avoiding multiple round trips for bulk operations like AI plan application.

### Returns

`Ok(())` on success; `Err(String)` on I/O failure.

### Implementation

1. Construct `ProjectPaths`.
2. Acquire `edit::lock::doc_lock` for the folder - the same lock `apply_edit_op` and (internally) `get_edit`'s `load_or_seed` take, so this write can't land torn between another writer's load and save. No `load_or_seed` call here at all (this is a blind overwrite, not a read-modify-write), so there's no double-locking risk to route around, unlike `apply_edit_op`.
3. Call `doc.save(&paths.edit())`.
