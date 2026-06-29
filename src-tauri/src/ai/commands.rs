use std::path::PathBuf;
use crate::edit::model::{EditDoc, Trim};
use crate::session::paths::ProjectPaths;

#[tauri::command]
pub fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };

    let mut doc = crate::edit::seed::load_or_seed(&paths);
    let dur_ms = doc.trim.out_ms;

    let log = crate::events::model::EventLog::load(&paths.events())
        .map_err(|e| e.to_string())?;
    let actions = crate::actions::model::ActionLog::load(&paths.actions())
        .map(|a| a.actions).unwrap_or_default();
    let cursor = crate::events::cursortype::CursorTrack::load(&paths.cursor());
    let typing = crate::events::typing::TypingLog::load(&paths.typing()).ms;

    let transcript = crate::ai::timeline::serialize(&log, &actions, &cursor, &typing, dur_ms);

    let model_name = model.unwrap_or_else(|| "llama3.2".into());
    let raw = crate::ai::ollama::chat(
        &model_name,
        &crate::ai::prompt::system_prompt(),
        &transcript,
    )?;

    let ops = crate::ai::plan::ops_from_json(&raw, dur_ms)?;

    // Replace mechanical auto-zooms with AI plan - only after both chat() and
    // ops_from_json() succeed (their ? exits early on failure).
    doc.zooms.clear();
    doc.trim = Trim { in_ms: 0, out_ms: dur_ms };
    for op in ops {
        crate::edit::api::apply(&mut doc, op);
    }

    doc.save(&paths.edit()).map_err(|e| e.to_string())?;
    Ok(doc)
}
