// Tauri commands for the `.tcursor` project format: opening a project via a native file dialog,
// listing recently-opened projects, and resolving the cold-start file-association argv (a
// `.tcursor` path Windows hands the exe when a user double-clicks the file). All three funnel
// through `folder_from_manifest_path`, the one place a manifest FILE path becomes a project
// FOLDER - the editor always opens a folder, never the manifest file itself.
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;
use crate::session::project::recents::{self, RecentProject};

/// Cold-start launch target, populated once in `lib.rs`'s `setup()` from argv. `None` on a normal
/// launch (no `.tcursor` argument) or when the argument doesn't resolve to a real project.
/// NOTE (warm-launch follow-up): this only covers a fresh process start. If TCursor is already
/// running and the user double-clicks another `.tcursor` file, the OS launches a second process
/// rather than forwarding the path to this one; wiring that up needs `tauri-plugin-single-instance`
/// and is deliberately left for a follow-up rather than half-done here.
#[derive(Default)]
pub struct LaunchProject(pub Option<String>);

/// Resolve the cold-start argv (if any) into a project folder. Pure/testable: takes an iterator
/// rather than reading `std::env::args()` itself. Only `argv[1]` is considered - Windows file
/// associations pass the associated file as the sole extra argument; anything else (dev-mode
/// flags, no argument at all) means "nothing to open".
pub fn launch_project_from_argv<I: Iterator<Item = String>>(mut args: I) -> Option<String> {
    args.next(); // argv[0]: the exe path itself
    let arg = args.next()?;
    if !arg.to_ascii_lowercase().ends_with(".tcursor") {
        return None;
    }
    folder_from_manifest_path(Path::new(&arg)).ok()
}

/// Shared core: a `.tcursor` file path -> its containing project folder. The manifest is read
/// (via `load_or_default`) purely to keep the read path exercised - `get_project_manifest` below
/// is the actual "already preprocessed?" check, called separately (by folder, not manifest path)
/// once the editor has a folder to open - a missing/corrupt manifest never blocks resolving the
/// folder here, which is all `open_project`'s caller actually needs today.
pub fn folder_from_manifest_path(manifest_path: &Path) -> Result<String, String> {
    let folder = manifest_path
        .parent()
        .filter(|p| p.is_dir())
        .ok_or_else(|| format!("no project folder next to {}", manifest_path.display()))?;
    let paths = ProjectPaths { folder: folder.to_path_buf() };
    let _ = ProjectManifest::load_or_default(&paths.manifest());
    Ok(folder.to_string_lossy().into_owned())
}

/// Opens a native file-picker filtered to `*.tcursor`, resolves the picked file to its
/// containing folder, and records it in the recents list.
#[tauri::command]
pub async fn open_project(app: tauri::AppHandle) -> Result<String, String> {
    // Same base dir `start_recording` writes new projects under, so the picker opens where
    // projects actually live instead of the OS's generic default (e.g. Documents).
    let default_dir = dirs_next::video_dir().unwrap_or_else(std::env::temp_dir).join("TCursor");
    let picked = app
        .dialog()
        .file()
        .add_filter("TCursor Project", &["tcursor"])
        .set_title("Open TCursor Project")
        .set_directory(default_dir)
        .blocking_pick_file()
        .ok_or_else(|| "no file selected".to_string())?;
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let folder = folder_from_manifest_path(&path)?;
    recents::touch(&folder);
    Ok(folder)
}

/// The small "recently opened" list (most-recent-first), for a future recents UI.
#[tauri::command]
pub fn list_recent_projects() -> Vec<RecentProject> {
    recents::list()
}

/// Consumes the cold-start launch target resolved in `setup()`. The frontend calls this once on
/// mount; a `Some` folder routes straight to the editor instead of showing the HUD.
#[tauri::command]
pub fn get_launch_project(state: tauri::State<'_, LaunchProject>) -> Option<String> {
    state.0.clone()
}

/// Reads the `project.tcursor` manifest for `folder` (the same back-compat default
/// `folder_from_manifest_path` falls back to when one is missing/corrupt). `useEditorData` calls
/// this once per folder to check `preprocessed` before deciding whether to run its own lazy
/// `ensure_*` fallback.
#[tauri::command]
pub fn get_project_manifest(folder: String) -> ProjectManifest {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    ProjectManifest::load_or_default(&paths.manifest())
}

/// Whether this recording's video already contains a baked OS cursor - derived from the
/// record-time `settings.json` snapshot, so it stays true for recordings made before this
/// existed. A derived read, deliberately NOT a field on the persisted manifest.
#[tauri::command]
pub fn os_cursor_in_video(folder: String) -> bool {
    crate::settings::store::os_cursor_in_video(&ProjectPaths { folder: PathBuf::from(&folder) })
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
