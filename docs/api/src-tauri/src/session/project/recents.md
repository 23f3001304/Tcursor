# src-tauri/src/session/project/recents.rs

A small "recently opened" list, persisted next to `settings::store`'s `config.json` (same app-data convention: `dirs_next::config_dir()`, falling back to a temp dir). Tracks projects the user has explicitly opened (via `open_project`'s dialog) and projects `stop_recording` just finished writing, so a future recents UI has something to show. Best-effort throughout: a write failure here must never block a project from opening or a recording from finishing.

## RecentProject

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecentProject {
    pub folder: String,
    pub name: String,
    pub opened_unix_ms: u64,
}
```

One entry in the recents list, as returned to the frontend by `list_recent_projects`.

- `folder: String` - absolute project directory path, the same value `open_project`/`stop_recording` hand to `onEdit`.
- `name: String` - the folder's own last path component (e.g. `"rec-1737910230123"`), derived once at `touch` time rather than re-derived by every caller.
- `opened_unix_ms: u64` - wall-clock time this entry was (re)touched, used to keep the list ordered most-recent-first.

### Used by

- `src-tauri/src/session/project/commands.rs` (`list_recent_projects`) - returns `recents::list()` directly.
- `src/lib/ipc.ts` (`RecentProject`) - the TS mirror (not yet rendered by any component).

## list

```rust
pub fn list() -> Vec<RecentProject>
```

Every recent entry, most-recently-opened first.

### Returns

`Vec<RecentProject>` from `recents_path()`, or an empty vec if the file is missing, unreadable, or corrupt - a fresh install with no recents yet is not an error.

### Behaviors

- `recents_path_is_under_tcursor` - confirms the path ends in `recents.json` under a `TCursor` folder.

## touch

```rust
pub fn touch(folder: &str)
```

Moves (or inserts) `folder` to the front of the recents list, capped at `MAX_RECENTS` (10) entries.

### Inputs

- `folder: &str` - absolute project directory path.

### Implementation

1. Derive `name` from `folder`'s `Path::file_name()`, falling back to the full `folder` string if unrepresentable as UTF-8.
2. `list()`, then `retain` to drop any prior entry for the same folder (so it never appears twice with a stale timestamp).
3. Insert the fresh entry at index 0, `truncate` to `MAX_RECENTS`.
4. Best-effort write: `create_dir_all` the parent, then `serde_json::to_vec_pretty` + `fs::write`; any failure at either step is silently swallowed.

### Behaviors

- `touch_moves_an_existing_folder_to_the_front_without_duplicating_it` - exercises the pure retain-then-insert logic directly (the real `touch` writes to the actual app-data `recents.json`, so - mirroring `pack_import_tests.rs`'s convention - the write side effect itself is not driven from a unit test).
- `truncates_to_max_recents` - confirms the cap logic keeps exactly `MAX_RECENTS` entries.

### Used by

- `src-tauri/src/session/project/commands.rs` (`open_project`) - touches the folder after a successful pick.
- `src-tauri/src/session/record/recorder.rs` (`stop_recording`) - touches the folder for every finished recording, best-effort, so fresh recordings appear in "recent" even without being manually reopened.
