# src-tauri/src/session/project/commands.rs

Tauri commands for the `.tcursor` project format: opening a project via a native file dialog, listing recently-opened projects, and resolving the cold-start file-association argv (a `.tcursor` path Windows hands the exe when a user double-clicks the file). All three funnel through `folder_from_manifest_path`, the one place a manifest FILE path becomes a project FOLDER - the editor always opens a folder, never the manifest file itself.

## LaunchProject

```rust
#[derive(Default)]
pub struct LaunchProject(pub Option<String>);
```

Tauri managed-state singleton holding the cold-start launch target.

- `pub Option<String>` - `Some(folder)` when this process was launched with a `.tcursor` argv path that resolved to a real folder; `None` on a normal launch or an argument that didn't resolve.

*Warm-launch is not handled:* this only covers a fresh process start. If TCursor is already running and the user double-clicks another `.tcursor` file, the OS launches a **second process** rather than forwarding the path to the running one. Wiring that up needs `tauri-plugin-single-instance` (not currently a dependency) to receive the second launch's argv in the first process and emit an event the frontend listens for; that is deliberately left as a follow-up rather than half-implemented here.

### Used by

- `src-tauri/src/lib.rs` (`setup`) - constructed from `launch_project_from_argv(std::env::args())` and registered via `app.manage(...)`.
- `get_launch_project` (this file) - reads it back for the frontend.

## launch_project_from_argv

```rust
pub fn launch_project_from_argv<I: Iterator<Item = String>>(mut args: I) -> Option<String>
```

Resolve the cold-start argv (if any) into a project folder. Pure and testable: takes an iterator rather than reading `std::env::args()` itself.

### Inputs

- `args: I` - typically `std::env::args()`. Only `args[1]` (the first argument after the exe path) is considered - Windows file associations pass the associated file as the sole extra argument; a dev-mode flag or no argument at all means "nothing to open".

### Returns

`Some(folder)` when `args[1]` ends with `.tcursor` (case-insensitive) and `folder_from_manifest_path` resolves it to a real folder. `None` otherwise - including when the path doesn't end in `.tcursor`, when there is no second argument, or when resolution fails (e.g. the folder no longer exists).

### Behaviors

- `launch_project_from_argv_resolves_a_tcursor_path`
- `launch_project_from_argv_is_case_insensitive_about_the_extension`
- `launch_project_from_argv_ignores_missing_or_non_tcursor_args`

## folder_from_manifest_path

```rust
pub fn folder_from_manifest_path(manifest_path: &Path) -> Result<String, String>
```

Shared core: a `.tcursor` file path -> its containing project folder. The manifest is read (via `ProjectManifest::load_or_default`) purely to keep the read path exercised - `get_project_manifest` (below) is the actual "already preprocessed?" check, called separately (by folder, not manifest path) once the editor has a folder to open - a missing/corrupt manifest never blocks resolving the folder here, which is all `open_project`'s caller actually needs today.

### Inputs

- `manifest_path: &Path` - the picked/opened `.tcursor` file's path.

### Returns

`Ok(folder)` (the absolute containing directory, as a `String`) when `manifest_path` has a parent directory that exists. `Err(String)` otherwise (e.g. the folder was deleted after the file was picked).

### Implementation

1. `manifest_path.parent()`, filtered to an existing directory; error if absent.
2. Build a `ProjectPaths` for that folder and call `ProjectManifest::load_or_default(&paths.manifest())` - discarded (`let _ =`) today, but keeps this the one place a manifest is read during project resolution.
3. Return the folder as a `String` (`to_string_lossy`).

### Behaviors

- `resolves_the_containing_folder_when_a_manifest_is_present`
- `falls_back_when_the_folder_has_no_manifest_at_all` - back-compat: a folder with no `project.tcursor` at all (an existing pre-feature recording) still resolves.
- `errors_when_the_parent_folder_does_not_exist`

### Used by

- `open_project` (this file), `launch_project_from_argv` (this file).

## open_project

```rust
#[tauri::command]
pub async fn open_project(app: tauri::AppHandle) -> Result<String, String>
```

Opens a native file-picker filtered to `*.tcursor`, resolves the picked file to its containing folder, and records it in the recents list.

### Inputs

- `app: tauri::AppHandle` - used to reach the dialog plugin via `tauri_plugin_dialog::DialogExt::dialog()`.

### Returns

`Ok(folder)` - the containing project folder, ready to pass to the same `onEdit`/`Editor` path the record/stop flow uses. `Err(String)` if the user cancels the dialog, the picked path can't be converted to a filesystem path, or `folder_from_manifest_path` fails.

### Implementation

1. `app.dialog().file().add_filter("TCursor Project", &["tcursor"]).set_title(...).set_directory(...).blocking_pick_file()`. The starting directory is `dirs_next::video_dir()/TCursor` (falling back to a temp dir) - the exact base `start_recording` writes new projects under, so the picker opens where projects actually live rather than the OS's generic default. *Why `async fn` + the blocking API:* this is the pattern `tauri-plugin-dialog` itself documents for command handlers - `blocking_pick_file` parks the calling (async, off-main-thread) task on a channel while the dialog runs on the main thread via `run_on_main_thread`, so it neither blocks the UI nor needs a callback.
2. `FilePath::into_path()` to get a `PathBuf`.
3. `folder_from_manifest_path(&path)`.
4. `recents::touch(&folder)` best-effort, then return the folder.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/lib/ipc.ts` (`openProject`) - the TS wrapper.
- `src/hud/Hud.tsx` (`openExistingProject`) - calls it from the HUD's "Open Project" button, then `onEdit`s the returned folder.

## list_recent_projects

```rust
#[tauri::command]
pub fn list_recent_projects() -> Vec<RecentProject>
```

The small "recently opened" list (most-recent-first), for a future recents UI.

### Returns

`recents::list()` directly.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/lib/ipc.ts` (`listRecentProjects`) - the TS wrapper (not yet rendered by any component).

## get_launch_project

```rust
#[tauri::command]
pub fn get_launch_project(state: tauri::State<'_, LaunchProject>) -> Option<String>
```

Consumes the cold-start launch target resolved in `setup()`.

### Inputs

- `state: tauri::State<'_, LaunchProject>` - the managed-state singleton populated once at startup.

### Returns

`state.0.clone()` - `Some(folder)` exactly once per process launch if argv named a resolvable `.tcursor` file, `None` otherwise. Calling it more than once returns the same value every time (it is not consumed/cleared) since the frontend only needs to read it once on mount.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/lib/ipc.ts` (`getLaunchProject`) - the TS wrapper.
- `src/App.tsx` - calls it once on mount; a non-null folder routes straight to the editor instead of the HUD.

## get_project_manifest

```rust
#[tauri::command]
pub fn get_project_manifest(folder: String) -> ProjectManifest
```

Reads the `project.tcursor` manifest for `folder`.

### Inputs

- `folder: String` - absolute project directory (NOT a manifest file path - contrast `folder_from_manifest_path` above).

### Returns

`ProjectManifest::load_or_default(&paths.manifest())` - the real manifest when present, else the synthesized "unknown source" default (never fails).

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/lib/ipc.ts` (`getProjectManifest`) - the TS wrapper.
- `src/editor/hooks/useEditorData.ts` - fetched once per `[folder]` into `preprocessed`, to decide whether to skip its own lazy `ensure_*` calls.

## os_cursor_in_video

```rust
#[tauri::command]
pub fn os_cursor_in_video(folder: String) -> bool
```

Whether `folder`'s recorded video already contains a baked OS cursor. A thin wrapper over `settings::store::os_cursor_in_video`, which derives it from the record-time `settings.json` snapshot.

*Why an IPC of its own rather than a field on `ProjectManifest`:* the manifest is a persisted file written at record-stop, so a new field would be absent on every existing recording and would need a migration and a default. Deriving the value at read time is correct for all recordings, old and new, and keeps a derived value out of a persisted struct.

*Who calls it:* `useEditorData` fetches it once per folder (alongside `cursorKinds` - both are properties of the recording, not of the doc) and hands it to `Stage` (which mirrors the renderer's plain-OS fallback in the canvas preview) and to `CursorPanel` (which annotates the System option when the cursor has to be re-created).

