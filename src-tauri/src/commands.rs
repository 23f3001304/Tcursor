use serde::Serialize;

#[derive(Serialize)]
pub struct DisplayInfo { pub id: u32, pub label: String }

#[derive(Serialize)]
pub struct AudioInfo { pub id: String, pub label: String }

#[tauri::command]
pub fn list_displays() -> Vec<DisplayInfo> {
    vec![DisplayInfo { id: 0, label: "Primary Display".into() }]
}

#[tauri::command]
pub fn list_audio_inputs() -> Vec<AudioInfo> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| it.filter_map(|d| {
            let name = d.name().ok()?;
            Some(AudioInfo { id: name.clone(), label: name })
        }).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_webcam(folder: String, bytes: Vec<u8>) -> Result<(), String> {
    let path = std::path::Path::new(&folder).join("webcam.webm");
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_project(folder: String, app: tauri::AppHandle) -> Result<(), String> {
    crate::export::pipeline::run::run_export(app, folder);
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

/// Toggle whether the app window appears in screen capture / screenshots. The HUD
/// stays excluded (hidden from recordings); the editor calls this to opt back in.
/// Resolves the "main" window via the app handle so it targets the exact HWND the
/// startup exclusion was applied to (avoids any window-handle mismatch).
#[tauri::command]
pub fn set_capturable(app: tauri::AppHandle, capturable: bool) -> bool {
    #[cfg(windows)]
    {
        use tauri::Manager;
        if let Some(win) = app.get_webview_window("main") {
            if let Ok(h) = win.hwnd() {
                return crate::win::capture_exclusion::set_capture_exclusion(h.0 as isize, !capturable);
            }
        }
        false
    }
    #[cfg(not(windows))]
    { let _ = (app, capturable); false }
}
