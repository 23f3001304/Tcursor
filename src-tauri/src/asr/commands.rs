use crate::asr::download::download_model;
use crate::asr::models::{find, is_installed, MODELS};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize)]
pub struct ModelDto {
    pub id: String,
    pub label: String,
    pub bytes: u64,
    pub installed: bool,
    pub multilingual: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct DownloadProgress {
    pub id: String,
    pub done: u64,
    pub total: u64,
}

static DOWNLOADING: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn claim(id: &str) -> bool {
    let mut busy = DOWNLOADING.lock().unwrap_or_else(|e| e.into_inner());
    if busy.iter().any(|x| x == id) {
        return false;
    }
    busy.push(id.to_string());
    true
}

fn release(id: &str) {
    let mut busy = DOWNLOADING.lock().unwrap_or_else(|e| e.into_inner());
    busy.retain(|x| x != id);
}

#[tauri::command]
pub fn whisper_models() -> Vec<ModelDto> {
    MODELS
        .iter()
        .map(|m| ModelDto {
            id: m.id.to_string(),
            label: m.label.to_string(),
            bytes: m.bytes,
            installed: is_installed(m.id),
            multilingual: m.multilingual,
        })
        .collect()
}

#[tauri::command]
pub fn download_whisper_model(id: String, app: AppHandle) {
    let Some(spec) = find(&id) else {
        let _ = app.emit(
            "asr-download-error",
            format!("Unknown caption model {id:?}."),
        );
        return;
    };
    if !claim(&id) {
        return;
    }
    std::thread::spawn(move || {
        let app_progress = app.clone();
        let id_progress = id.clone();
        let on_progress = move |done: u64, total: u64| {
            let _ = app_progress.emit(
                "asr-download-progress",
                DownloadProgress {
                    id: id_progress.clone(),
                    done,
                    total,
                },
            );
        };
        let result = download_model(spec, &on_progress, &|| false);
        release(&id);
        match result {
            Ok(_) => {
                let _ = app.emit("asr-download-done", id);
            }
            Err(e) => {
                let _ = app.emit("asr-download-error", e);
            }
        }
    });
}

#[derive(serde::Serialize, Clone)]
pub struct Progress {
    pub phase: String,
    pub pct: u32,
}

static TRANSCRIBING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn busy() -> &'static Mutex<HashSet<String>> {
    TRANSCRIBING.get_or_init(Default::default)
}

fn running(folder: &str) -> bool {
    busy()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(folder)
}

fn start(folder: &str) -> bool {
    busy()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(folder.to_string())
}

fn finish(folder: &str) {
    busy()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(folder);
}

fn progress(app: &AppHandle, phase: &str, pct: u32) {
    let _ = app.emit(
        "asr-progress",
        Progress {
            phase: phase.to_string(),
            pct,
        },
    );
}

#[tauri::command]
pub fn transcribe_project(folder: String, app: AppHandle) {
    if !start(&folder) {
        return;
    }
    std::thread::spawn(move || {
        let outcome = run(&folder, &app);
        finish(&folder);
        match outcome {
            Ok(n) => {
                let _ = app.emit("asr-done", n);
            }
            Err(e) => {
                let _ = app.emit("asr-error", e);
            }
        }
    });
}

fn run(folder: &str, app: &AppHandle) -> Result<u32, String> {
    let paths = crate::session::paths::ProjectPaths {
        folder: std::path::PathBuf::from(folder),
    };
    let style = crate::edit::seed::load_or_seed(&paths).settings.captions;
    let spec = crate::asr::models::resolve_model(&style)?;
    if !is_installed(spec.id) {
        return Err(format!(
            "{} is not downloaded yet. Download it from the Captions panel.",
            spec.label
        ));
    }
    let src = crate::asr::audio::pick_source(&paths)
        .ok_or("This recording has no audio to transcribe.")?;
    progress(app, "decode", 0);
    let vad_model =
        crate::asr::download::download_model(&crate::asr::models::VAD_MODEL, &|_, _| {}, &|| {
            !running(folder)
        })
        .ok();
    let pcm = crate::asr::audio::decode_16k_mono(&crate::asr::audio::wav_for(&paths, src))?;
    progress(app, "decode", 100);
    let params = crate::asr::whisper::AsrParams {
        model_path: crate::asr::models::model_path(spec.id),
        language: style.language.clone(),
        threads: crate::asr::whisper::default_threads(),
        vad_model,
    };
    let tokens = crate::asr::whisper::transcribe(
        &pcm,
        &params,
        &|pct| progress(app, "transcribe", pct),
        &|| !running(folder),
    )?;
    let words = crate::asr::audio::shift_words(
        crate::asr::words::words_from_tokens(&tokens),
        crate::asr::audio::source_shift_ms(&paths, src),
    );
    let captions =
        crate::asr::group::group_words(&words, &crate::asr::group::GroupLimits::default());
    let count = captions.len() as u32;
    crate::edit::commands::apply_edit_op(
        folder.to_string(),
        crate::edit::ops::api::EditOp::SetCaptions { captions },
    )?;
    Ok(count)
}
