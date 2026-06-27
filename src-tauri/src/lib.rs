pub mod domain;
pub mod capture;
pub mod encode;
pub mod audio;
pub mod session;
pub mod win;
pub mod commands;
pub mod events;
pub mod actions;
pub mod export;
pub mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(session::recorder::Recorder::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_displays,
            commands::list_audio_inputs,
            session::recorder::start_recording,
            session::recorder::pause_recording,
            session::recorder::resume_recording,
            session::recorder::stop_recording,
            commands::save_webcam,
            commands::export_project,
            commands::get_settings,
            commands::set_settings,
        ])
        .setup(|app| {
            use tauri::Manager;
            // Use the bundled ffmpeg/ffprobe (installed builds); dev falls back to PATH.
            if let Ok(res) = app.path().resource_dir() {
                for cand in [res.join("resources"), res] {
                    if cand.join("ffmpeg.exe").exists() {
                        crate::win::proc::set_ffmpeg_dir(cand);
                        break;
                    }
                }
            }
            // Probe the encoder off-thread now so the first recording's ffmpeg sink
            // is fast — audio capture must not start behind a slow first ffmpeg launch.
            std::thread::spawn(crate::encode::ffmpeg_encoder::prewarm);
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
