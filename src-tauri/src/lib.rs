pub mod domain;
pub mod capture;
pub mod encode;
pub mod audio;
pub mod session;
pub mod win;
pub mod commands;
pub mod events;
pub mod export;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::Recorder::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_displays,
            commands::list_audio_inputs,
            commands::start_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::stop_recording,
            commands::save_webcam,
            commands::export_project,
        ])
        .setup(|app| {
            use tauri::Manager;
            if let Some(win) = app.get_webview_window("main") {
                #[cfg(windows)]
                {
                    // Hides the HUD from screen capture/recordings. Set false temporarily
                    // if you need to screenshot the HUD during design work.
                    const CAPTURE_EXCLUDE: bool = true;
                    if CAPTURE_EXCLUDE {
                        if let Ok(hwnd) = win.hwnd() {
                            let ok = win::capture_exclusion::exclude_from_capture(hwnd.0 as isize);
                            if ok {
                                println!("capture exclusion applied");
                            } else {
                                eprintln!("WARNING: capture exclusion FAILED — HUD may appear in recordings");
                            }
                        }
                    } else {
                        println!("capture exclusion DISABLED (dev) — HUD is screenshottable");
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
