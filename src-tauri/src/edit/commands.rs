use crate::edit::model::{EditDoc};
use crate::edit::ops::api::EditOp;
use crate::session::paths::ProjectPaths;

fn paths(folder: &str) -> ProjectPaths {
    ProjectPaths { folder: std::path::PathBuf::from(folder) }
}

#[tauri::command]
pub fn get_edit(folder: String) -> Result<EditDoc, String> {
    let p = paths(&folder);
    Ok(crate::edit::seed::load_or_seed(&p))
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
