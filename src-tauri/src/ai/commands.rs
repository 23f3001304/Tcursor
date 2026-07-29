use std::path::PathBuf;
use crate::edit::model::{EditDoc, Trim};
use crate::edit::ops::api::EditOp;
use crate::events::model::EventLog;
use crate::session::paths::ProjectPaths;

/// One labeled step of the AI director's plan, revealed one-at-a-time by the agentic editor UI.
#[derive(serde::Serialize)]
pub struct AiStep { pub op: EditOp, pub label: String }

/// List locally-installed Ollama models, for the AI panel's Engine picker (`AiPanel.tsx`).
/// Empty (never an error) when Ollama isn't running - the picker then just shows the backend's
/// own fallback name instead of a populated list.
#[tauri::command]
pub fn list_ollama_models() -> Vec<String> {
    crate::ai::backend::ollama::list_models()
}

/// Shared LLM pass: load the recording, serialize it to a transcript, pick an actually-installed
/// chat model, and parse the model's JSON into edit ops. Returns the current doc + ops + event log
/// (for narration) + true clip length. Both `ai_autoedit` and `ai_plan` build on this.
fn build_plan(paths: &ProjectPaths, model: Option<String>)
    -> Result<(EditDoc, Vec<EditOp>, EventLog, u32), String> {
    let doc = crate::edit::seed::load_or_seed(paths);
    // TRUE length - NOT doc.trim.out_ms, which is 0 after a trim reset (0 == "whole clip"), which
    // would otherwise feed the AI a zero-length timeline.
    let dur_ms = match crate::edit::seed::true_duration_ms(paths) { 0 => doc.trim.out_ms, t => t };
    let log = EventLog::load(&paths.events()).map_err(|e| e.to_string())?;
    let actions = crate::actions::model::ActionLog::load(&paths.actions()).map(|a| a.actions).unwrap_or_default();
    let cursor = crate::events::track::cursortype::CursorTrack::load(&paths.cursor());
    let typing = crate::events::track::typing::TypingLog::load(&paths.typing()).ms;
    let transcript = crate::ai::backend::timeline::serialize(&log, &actions, &cursor, &typing, dur_ms);
    // Actually-installed chat model: the caller's pick if present, else the first installed one -
    // never a hardcoded name (Ollama 404s on models that aren't pulled).
    let installed = crate::ai::backend::ollama::list_models();
    let model_name = model
        .filter(|m| !m.is_empty() && installed.iter().any(|i| i == m))
        .or_else(|| installed.first().cloned())
        .ok_or_else(|| "No Ollama models are installed. Pull one first, e.g.: ollama pull llama3.2".to_string())?;
    let raw = crate::ai::backend::ollama::chat(&model_name, &crate::ai::backend::prompt::system_prompt(), &transcript)?;
    let ops = crate::ai::backend::plan::ops_from_json(&raw, dur_ms)?;
    Ok((doc, ops, log, dur_ms))
}

/// One-shot: apply the whole AI plan and return the final doc (non-agentic path).
#[tauri::command]
pub fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    let (mut doc, ops, _log, dur_ms) = build_plan(&paths, model)?;
    doc.zooms.clear();
    doc.trim = Trim { in_ms: 0, out_ms: dur_ms };
    for op in ops { crate::edit::ops::api::apply(&mut doc, op); }
    doc.save(&paths.edit()).map_err(|e| e.to_string())?;
    Ok(doc)
}

/// Agentic: return the plan as ordered, labeled steps WITHOUT applying anything. The frontend
/// applies them one-at-a-time (each via `apply_edit_op`) to reveal the edit like a live agent.
#[tauri::command]
pub fn ai_plan(folder: String, model: Option<String>) -> Result<Vec<AiStep>, String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    let (doc, ops, log, dur_ms) = build_plan(&paths, model)?;
    let mut steps = Vec::new();
    // Open by clearing the mechanical auto-zooms (only when there are any) so the reveal shows them
    // giving way to the smart ones.
    if !doc.zooms.is_empty() {
        steps.push(AiStep { op: EditOp::ClearZooms, label: "Rethinking your zooms…".into() });
    }
    for op in ops {
        let label = crate::ai::backend::narrate::label_for(&op, &log, dur_ms);
        steps.push(AiStep { op, label });
    }
    Ok(steps)
}
