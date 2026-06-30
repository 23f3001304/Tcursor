pub mod ai;
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
pub mod edit;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(session::recorder::Recorder::default())
        .manage(export::preview::PreviewSession::default())
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
            edit::commands::get_edit,
            edit::commands::apply_edit_op,
            edit::commands::save_edit,
            ai::commands::ai_autoedit,
            export::preview::preview_frame,
            export::preview::preview_bg,
            export::preview_track::camera_track,
            export::preview_track::preview_layout,
            export::preview_track::click_track,
            export::preview_track::spotlight_holds,
            export::preview_track::ensure_proxy,
            export::cursorpreview::cursor_sprites,
            export::cursorpreview::cursor_kinds,
            export::thumbs::ensure_thumbs,
            export::thumbs::ensure_waveform,
            export::thumbs::ensure_preview_audio,
            commands::set_capturable,
        ])
        .setup(|app| {
            use tauri::Manager;
            // Resolve the bundled ffmpeg/ffprobe from the app exe dir (robust on
            // installed builds where resource_dir() may not); dev falls back to PATH.
            // The diagnostic log explains "ffmpeg not available" failures on any PC.
            let diag = crate::win::proc::init_ffmpeg(app.path().resource_dir().ok());
            let _ = std::fs::write(std::env::temp_dir().join("tcursor-ffmpeg.log"), diag);
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
                            let ok = win::capture_exclusion::set_capture_exclusion(hwnd.0 as isize, true);
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
