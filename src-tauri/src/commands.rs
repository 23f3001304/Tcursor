use serde::Serialize;

#[derive(Serialize)]
pub struct DisplayInfo { pub id: String, pub label: String, pub kind: String }

#[derive(Serialize)]
pub struct AudioInfo { pub id: String, pub label: String }

#[tauri::command]
pub fn list_displays() -> Vec<DisplayInfo> {
    let mut targets = Vec::new();

    if let Ok(monitors) = windows_capture::monitor::Monitor::enumerate() {
        for (i, m) in monitors.iter().enumerate() {
            let name = m.name().unwrap_or_else(|_| format!("Display {}", i + 1));
            let w = m.width().unwrap_or(0);
            let h = m.height().unwrap_or(0);
            let label = if i == 0 {
                format!("Display {}: {} (Primary)", i + 1, name)
            } else if w > 0 && h > 0 {
                format!("Display {}: {} ({}x{})", i + 1, name, w, h)
            } else {
                format!("Display {}: {}", i + 1, name)
            };
            targets.push(DisplayInfo {
                id: format!("display:{i}"),
                label,
                kind: "display".into(),
            });
        }
    }

    if targets.is_empty() {
        targets.push(DisplayInfo {
            id: "display:0".into(),
            label: "Primary Display".into(),
            kind: "display".into(),
        });
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
            GWL_EXSTYLE, WS_EX_TOOLWINDOW,
        };

        struct WinEnumContext {
            list: Vec<DisplayInfo>,
        }

        unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut WinEnumContext);
            if IsWindowVisible(hwnd).as_bool() {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                    if (ex_style & WS_EX_TOOLWINDOW.0) == 0 {
                        let mut buf = vec![0u16; (len + 1) as usize];
                        let read = GetWindowTextW(hwnd, &mut buf);
                        if read > 0 {
                            let title = String::from_utf16_lossy(&buf[..read as usize]).trim().to_string();
                            if !title.is_empty()
                                && title != "Program Manager"
                                && title != "Settings"
                                && title != "TCursor"
                                && !title.starts_with("MSCTFIME")
                            {
                                ctx.list.push(DisplayInfo {
                                    id: format!("window:0x{:x}", hwnd.0 as usize),
                                    label: format!("App: {}", title),
                                    kind: "window".into(),
                                });
                            }
                        }
                    }
                }
            }
            BOOL(1)
        }

        let mut ctx = WinEnumContext { list: Vec::new() };
        unsafe {
            let _ = EnumWindows(Some(enum_windows_callback), LPARAM(&mut ctx as *mut _ as isize));
        }

        targets.extend(ctx.list);
    }

    targets
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

/// Append one MediaRecorder chunk to webcam.webm during recording (streamed via 1s timeslices), so
/// Stop has almost nothing left to write instead of one O(clip-length) blob. The recording folder
/// is freshly created per recording, so the first append creates the file.
#[tauri::command]
pub fn append_webcam(folder: String, bytes: Vec<u8>) -> Result<(), String> {
    use std::io::Write;
    let path = std::path::Path::new(&folder).join("webcam.webm");
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_project(folder: String, settings: crate::export::settings::ExportSettings, app: tauri::AppHandle) -> Result<(), String> {
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
                return crate::win::sys::capture_exclusion::set_capture_exclusion(h.0 as isize, !capturable);
            }
        }
        false
    }
    #[cfg(not(windows))]
    { let _ = (app, capturable); false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_targets() {
        use windows_capture::monitor::Monitor;
        use windows_capture::window::Window;

        let targets = list_displays();
        println!("--- TARGETS ({}) ---", targets.len());
        for t in &targets {
            println!("  [{}] {} ({})", t.kind, t.label, t.id);
        }

        if let Ok(m) = Monitor::from_index(0) {
            println!("Monitor 0 ok: {}x{}", m.width().unwrap_or(0), m.height().unwrap_or(0));
        }

        if let Some(win_target) = targets.iter().find(|t| t.kind == "window") {
            if let Some(hex) = win_target.id.strip_prefix("window:0x") {
                if let Ok(hwnd_val) = usize::from_str_radix(hex, 16) {
                    let hwnd = windows::Win32::Foundation::HWND(hwnd_val as *mut _);
                    let w = Window::from_raw_hwnd(hwnd.0 as *mut std::ffi::c_void);
                    println!("Window from_raw_hwnd ok: title={:?}", w.title());
                }
            }
        }
    }
}

