use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;
use crate::session::project::recents::{self, RecentProject};

#[derive(Default)]
pub struct LaunchProject(pub Option<String>);

pub fn launch_project_from_argv<I: Iterator<Item = String>>(mut args: I) -> Option<String> {
    args.next();
    let arg = args.next()?;
    if !arg.to_ascii_lowercase().ends_with(".tcursor") {
        return None;
    }
    folder_from_manifest_path(Path::new(&arg)).ok()
}

pub fn folder_from_manifest_path(manifest_path: &Path) -> Result<String, String> {
    let folder = manifest_path
        .parent()
        .filter(|p| p.is_dir())
        .ok_or_else(|| format!("no project folder next to {}", manifest_path.display()))?;
    let paths = ProjectPaths {
        folder: folder.to_path_buf(),
    };
    let _ = ProjectManifest::load_or_default(&paths.manifest());
    Ok(folder.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn open_project(app: tauri::AppHandle) -> Result<String, String> {
    let default_dir = dirs_next::video_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor");
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

#[tauri::command]
pub fn list_recent_projects() -> Vec<RecentProject> {
    recents::list()
}

#[tauri::command]
pub fn get_launch_project(state: tauri::State<'_, LaunchProject>) -> Option<String> {
    state.0.clone()
}

#[tauri::command]
pub fn get_project_manifest(folder: String) -> ProjectManifest {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    ProjectManifest::load_or_default(&paths.manifest())
}

#[tauri::command]
pub fn os_cursor_in_video(folder: String) -> bool {
    crate::settings::store::os_cursor_in_video(&ProjectPaths {
        folder: PathBuf::from(&folder),
    })
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
