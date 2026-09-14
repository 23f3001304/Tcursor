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

/// Guards against a second close-triggered stop stacking while `close_guard::finish_and_close` is
/// already running for an earlier `CloseRequested` (e.g. the OS delivering it again while the
/// window is on its way down). Process-lifetime static; never reset - the app is quitting either
/// way once this is set.
static CLOSING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(session::record::recorder::Recorder::default())
        .manage(export::preview::PreviewSession::default())
        .on_window_event(|window, event| {
            // R6: never lose a take to the close button. If a recording is active or its stop is
            // still finalizing (`Recorder::is_busy`), this is the safety net for the OS close
            // button / Alt+F4 / a wedged renderer - see `close_guard::finish_and_close`'s doc
            // comment for why it does not depend on the frontend for correctness. The HUD's own
            // Close button races this with a JS-side graceful stop first
            // (`useRecordingFlow.stopForClose`): by the time that resolves and calls
            // `window.close()`, `is_busy()` is already false and this arm never fires.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Manager;
                let recorder = window.state::<session::record::recorder::Recorder>();
                if recorder.is_busy() {
                    api.prevent_close();
                    if !CLOSING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn(session::record::close_guard::finish_and_close(app));
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_displays,
            commands::list_audio_inputs,
            session::record::recorder::start_recording,
            session::record::recorder::pause_recording,
            session::record::recorder::resume_recording,
            session::record::recorder_stop::stop_recording,
            session::record::switch_mic::switch_mic,
            session::record::switch_display::switch_display,
            commands::save_webcam,
            commands::append_webcam,
            session::record::webcam_segments::mark_webcam_segment,
            commands::export_project,
            commands::get_settings,
            commands::set_settings,
            edit::commands::get_edit,
            edit::commands::apply_edit_op,
            edit::commands::save_edit,
            ai::commands::ai_plan,
            ai::commands::list_ollama_models,
            export::preview::preview_frame,
            export::preview::preview_bg,
            export::preview::bg_thumbs::background_thumbs,
            export::preview::preview_track::camera_track,
            export::pipeline::silence::detect_silences,
            export::preview::preview_track::preview_layout,
            export::preview::preview_track::click_track,
            export::preview::preview_track::ensure_proxy,
            export::preview::preview_layouts::preview_layouts,
            export::preview::preview_fx::preview_fx_overlay,
            export::cursor::cursorpreview::cursor_sprites,
            export::cursor::cursorpreview::cursor_kinds,
            export::cursor::cursorpreview::cursor_layer,
            export::cursor::pack::list_cursor_packs,
            export::cursor::pack_import::import_cursor_pack,
            settings::bg_asset::import_background_asset,
            settings::bg_asset::background_asset_info,
            settings::bg_asset::remove_background_asset,
            export::preview::thumbs::ensure_thumbs,
            export::preview::thumbs::ensure_waveform,
            export::preview::thumbs::ensure_preview_audio,
            export::preview::preprocess::preprocess_project,
            commands::set_capturable,
            session::project::commands::open_project,
            session::project::commands::list_recent_projects,
            session::project::commands::get_launch_project,
            session::project::commands::get_project_manifest,
            session::project::commands::os_cursor_in_video,
        ])
        .setup(|app| {
            use tauri::Manager;
            // Cold-start file association: if argv[1] is a `.tcursor` path (Windows hands the
            // associated file to a fresh process on double-click), resolve it once here so the
            // frontend can route straight to the editor via `get_launch_project`. Warm-launch
            // (forwarding to an already-running instance) is NOT handled - see `LaunchProject`'s
            // doc comment.
            app.manage(session::project::commands::LaunchProject(
                session::project::commands::launch_project_from_argv(std::env::args()),
            ));
            // Resolve the bundled ffmpeg/ffprobe from the app exe dir (robust on
            // installed builds where resource_dir() may not); dev falls back to PATH.
            // The diagnostic log explains "ffmpeg not available" failures on any PC.
            let diag = crate::win::sys::proc::init_ffmpeg(app.path().resource_dir().ok());
            let _ = std::fs::write(std::env::temp_dir().join("tcursor-ffmpeg.log"), diag);
            // Probe the encoder off-thread now so the first recording's ffmpeg sink
            // is fast — audio capture must not start behind a slow first ffmpeg launch.
            std::thread::spawn(crate::encode::ffmpeg_encoder::prewarm);
            // The background picker's 53 thumbnails: loaded from the cache dir, or rendered once
            // and cached, before the editor can ask for them (see `bg_thumbs::prewarm`).
            std::thread::spawn(crate::export::preview::bg_thumbs::prewarm);
            if let Some(win) = app.get_webview_window("main") {
                #[cfg(windows)]
                {
                    // Hides the HUD from screen capture/recordings. Set false temporarily
                    // if you need to screenshot the HUD during design work.
                    const CAPTURE_EXCLUDE: bool = true;
                    if CAPTURE_EXCLUDE {
                        if let Ok(hwnd) = win.hwnd() {
                            let ok = win::sys::capture_exclusion::set_capture_exclusion(hwnd.0 as isize, true);
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
