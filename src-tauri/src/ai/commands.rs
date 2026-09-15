use crate::ai::llm::ollama::OllamaModel;
use crate::ai::plan::schema::AiRun;
use crate::session::paths::ProjectPaths;
use std::path::PathBuf;

#[tauri::command]
pub async fn list_ollama_models() -> Vec<OllamaModel> {
    tauri::async_runtime::spawn_blocking(|| {
        crate::ai::llm::ollama::list_models()
            .into_iter()
            .map(|name| OllamaModel {
                vision: crate::ai::llm::vision::has_vision(&name),
                name,
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

pub(crate) fn pick_model(
    requested: Option<String>,
    installed: &[String],
) -> Result<String, String> {
    requested
        .filter(|m| !m.is_empty() && installed.iter().any(|i| i == m))
        .or_else(|| installed.first().cloned())
        .ok_or_else(|| {
            "No Ollama models are installed. Pull one first, e.g.: ollama pull llama3.2".to_string()
        })
}

#[tauri::command]
pub async fn ai_propose(folder: String, model: Option<String>) -> Result<AiRun, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::ai::run::propose(
            &ProjectPaths {
                folder: PathBuf::from(folder),
            },
            model,
        )
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
        assert_eq!(
            pick_model(Some("mistral".into()), &installed),
            Ok("mistral".to_string())
        );
    }

    #[test]
    fn pick_model_falls_back_to_first_installed_when_choice_is_missing_empty_or_uninstalled() {
        let installed = vec!["llama3.2".to_string(), "mistral".to_string()];
        assert_eq!(pick_model(None, &installed), Ok("llama3.2".to_string()));
        assert_eq!(
            pick_model(Some(String::new()), &installed),
            Ok("llama3.2".to_string())
        );
        assert_eq!(
            pick_model(Some("not-pulled".into()), &installed),
            Ok("llama3.2".to_string())
        );
    }

    #[test]
    fn pick_model_errors_when_nothing_is_installed() {
        assert!(pick_model(None, &[]).is_err());
        assert!(pick_model(Some("llama3.2".into()), &[]).is_err());
    }
}
