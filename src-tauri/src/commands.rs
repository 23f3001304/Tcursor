use serde::Serialize;
use std::sync::Arc;

use crate::platform::Platform;
use crate::ports::capture::TargetKind;

#[derive(Serialize)]
pub struct DisplayInfo {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct AudioInfo {
    pub id: String,
    pub label: String,
}

#[tauri::command]
pub fn list_displays(platform: tauri::State<'_, Arc<Platform>>) -> Vec<DisplayInfo> {
    platform
        .capture
        .list_targets()
        .into_iter()
        .map(|t| DisplayInfo {
            id: t.id.to_string(),
            label: t.label,
            kind: match t.kind {
                TargetKind::Window => "window".into(),
                TargetKind::Display => "display".into(),
            },
        })
        .collect()
}

#[tauri::command]
pub fn list_audio_inputs() -> Vec<AudioInfo> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| {
            it.filter_map(|d| {
                let name = d.name().ok()?;
                Some(AudioInfo {
                    id: name.clone(),
                    label: name,
                })
            })
            .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn append_webcam(folder: String, bytes: Vec<u8>, segment: Option<u32>) -> Result<(), String> {
    use std::io::Write;
    let name = crate::session::record::webcam_segments::webcam_segment_name(segment);
    let path = std::path::Path::new(&folder).join(name);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_project(
    folder: String,
    settings: crate::export::settings::ExportSettings,
    app: tauri::AppHandle,
) -> Result<(), String> {
    crate::export::pipeline::run::run_export(app, folder, settings);
    Ok(())
}

#[tauri::command]
pub fn get_settings() -> crate::settings::model::Settings {
    crate::settings::store::load()
}

#[tauri::command]
pub fn set_settings(settings: crate::settings::model::Settings) -> Result<(), String> {
    crate::settings::store::save(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_capturable(
    app: tauri::AppHandle,
    capturable: bool,
    platform: tauri::State<'_, Arc<Platform>>,
) -> bool {
    use tauri::Manager;
    match app
        .get_webview_window("main")
        .as_ref()
        .and_then(crate::shell::window::handle)
    {
        Some(h) => platform.system.exclude_from_capture(h, !capturable),
        None => false,
    }
}
