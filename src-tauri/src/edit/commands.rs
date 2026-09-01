use crate::edit::model::{EditDoc};
use crate::edit::ops::api::EditOp;
use crate::session::paths::ProjectPaths;

fn paths(folder: &str) -> ProjectPaths {
    ProjectPaths { folder: std::path::PathBuf::from(folder) }
}

/// `async` + `spawn_blocking`: on a project that was never preprocessed (a legacy recording, or
/// one whose preprocess pass failed) `load_or_seed` falls into `seed::build_default`, which
/// gzip-decodes the whole event log, runs autozoom over every mouse sample and spawns up to two
/// `ffprobe` subprocesses to rebuild the timeline. This is the FIRST call the editor makes on
/// mount, so as a sync command that seed froze the window on the project-open path.
#[tauri::command]
pub async fn get_edit(folder: String) -> Result<EditDoc, String> {
    tauri::async_runtime::spawn_blocking(move || crate::edit::seed::load_or_seed(&paths(&folder)))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apply_edit_op(folder: String, op: EditOp) -> Result<EditDoc, String> {
    let p = paths(&folder);
    let mut doc = crate::edit::seed::load_or_seed(&p);
    crate::edit::ops::api::apply(&mut doc, op);
    doc.save(&p.edit()).map_err(|e| e.to_string())?;
    Ok(doc)
}

#[tauri::command]
pub fn save_edit(folder: String, doc: EditDoc) -> Result<(), String> {
    let p = paths(&folder);
    doc.save(&p.edit()).map_err(|e| e.to_string())
}
