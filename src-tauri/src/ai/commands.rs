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
///
/// `async` + `spawn_blocking`: `ollama::list_models` does a blocking HTTP call (`ureq`, no async
/// runtime). A plain sync `#[tauri::command] fn` runs on the WHOLE APP's main thread (Tauri v2
/// dispatches non-async commands inline on the thread that received the IPC message, not onto a
/// pool - see `tauri-macros::command::wrapper::body_blocking`), so a sync version of this would
/// freeze the window for the call's duration. Moving the blocking work into `spawn_blocking`
/// keeps the main thread free while it runs.
#[tauri::command]
pub async fn list_ollama_models() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(crate::ai::backend::ollama::list_models)
        .await
        .unwrap_or_default()
}

/// Picks the chat model `build_plan` sends to Ollama: the caller's choice if it's one of the
/// actually-installed models, else the first installed one - never a hardcoded name (Ollama 404s
/// on models that aren't pulled). Pure/no I/O so it's unit-testable without a live Ollama.
fn pick_model(requested: Option<String>, installed: &[String]) -> Result<String, String> {
    requested
        .filter(|m| !m.is_empty() && installed.iter().any(|i| i == m))
        .or_else(|| installed.first().cloned())
        .ok_or_else(|| "No Ollama models are installed. Pull one first, e.g.: ollama pull llama3.2".to_string())
}

/// Shared LLM pass: load the recording, serialize it to a transcript, pick an actually-installed
/// chat model, and parse the model's JSON into edit ops. Returns the current doc + ops + event log
/// (for narration) + true clip length. Both `ai_autoedit` and `ai_plan` build on this. Blocking
/// (file I/O + `ollama::chat`'s network call) - callers MUST run this inside `spawn_blocking`,
/// never directly on a command's async task.
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
    // Same shift `edit::seed`/`edit::migrate` use to put seeded regions on the output clock - the
    // director's ops are applied as output-time zooms, so its transcript must reason on that same
    // clock (reused, not re-derived, so the two can never disagree).
    let shift = crate::edit::seed::output_shift(paths);
    let transcript = crate::ai::backend::timeline::serialize(&log, &actions, &cursor, &typing, dur_ms, shift);
    let installed = crate::ai::backend::ollama::list_models();
    let model_name = pick_model(model, &installed)?;
    let raw = crate::ai::backend::ollama::chat(&model_name, &crate::ai::backend::prompt::system_prompt(), &transcript)?;
    let ops = crate::ai::backend::plan::ops_from_json(&raw, dur_ms)?;
    Ok((doc, ops, log, dur_ms))
}

/// One-shot: apply the whole AI plan and return the final doc (non-agentic path). `async` +
/// `spawn_blocking` for the same reason as `list_ollama_models` above - `build_plan`'s Ollama call
/// can take minutes on a first-run model load, and a sync command would freeze the app for all of
/// it.
#[tauri::command]
pub async fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = ProjectPaths { folder: PathBuf::from(folder) };
        let (mut doc, ops, _log, dur_ms) = build_plan(&paths, model)?;
        doc.zooms.clear();
        doc.trim = Trim { in_ms: 0, out_ms: dur_ms };
        for op in ops { crate::edit::ops::api::apply(&mut doc, op); }
        doc.save(&paths.edit()).map_err(|e| e.to_string())?;
        Ok(doc)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Agentic: return the plan as ordered, labeled steps WITHOUT applying anything. The frontend
/// applies them one-at-a-time (each via `apply_edit_op`) to reveal the edit like a live agent.
/// `async` + `spawn_blocking` - see `list_ollama_models`/`ai_autoedit` above; the frontend's
/// `useDirector` treats this fetch as its own cancellable "planning" phase (`Editor.md`).
#[tauri::command]
pub async fn ai_plan(folder: String, model: Option<String>) -> Result<Vec<AiStep>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = ProjectPaths { folder: PathBuf::from(folder) };
        let (doc, ops, log, dur_ms) = build_plan(&paths, model)?;
        let mut steps = Vec::new();
        // Open by clearing the mechanical auto-zooms (only when there are any) so the reveal
        // shows them giving way to the smart ones.
        if !doc.zooms.is_empty() {
            steps.push(AiStep { op: EditOp::ClearZooms, label: "Rethinking your zooms…".into() });
        }
        for op in ops {
            let label = crate::ai::backend::narrate::label_for(&op, &log, dur_ms);
            steps.push(AiStep { op, label });
        }
        Ok(steps)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_model_keeps_the_callers_choice_when_it_is_installed() {
        let installed = vec!["llama3.2".to_string(), "mistral".to_string()];
        assert_eq!(pick_model(Some("mistral".into()), &installed), Ok("mistral".to_string()));
    }

    #[test]
    fn pick_model_falls_back_to_first_installed_when_choice_is_missing_empty_or_uninstalled() {
        let installed = vec!["llama3.2".to_string(), "mistral".to_string()];
        assert_eq!(pick_model(None, &installed), Ok("llama3.2".to_string()));
        assert_eq!(pick_model(Some(String::new()), &installed), Ok("llama3.2".to_string()));
        assert_eq!(pick_model(Some("not-pulled".into()), &installed), Ok("llama3.2".to_string()));
    }

    #[test]
    fn pick_model_errors_when_nothing_is_installed() {
        assert!(pick_model(None, &[]).is_err());
        assert!(pick_model(Some("llama3.2".into()), &[]).is_err());
    }
}
