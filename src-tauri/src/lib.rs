pub mod actions;
pub mod ai;
pub mod asr;
pub mod audio;
pub mod capture;
pub mod commands;
pub mod domain;
pub mod edit;
pub mod encode;
pub mod events;
pub mod export;
pub mod platform;
pub mod ports;
pub mod process;
pub mod session;
pub mod settings;
pub mod shell;

static CLOSING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(session::record::recorder::Recorder::default())
        .manage(export::preview::PreviewSession::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Manager;
                let recorder = window.state::<session::record::recorder::Recorder>();
                if recorder.is_busy() {
                    api.prevent_close();
                    if !CLOSING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn(
                            session::record::close_guard::finish_and_close(app),
                        );
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
            commands::append_webcam,
            session::record::webcam_segments::mark_webcam_segment,
            commands::export_project,
            commands::get_settings,
            commands::set_settings,
            edit::commands::get_edit,
            edit::commands::apply_edit_op,
            edit::commands::save_edit,
            ai::commands::ai_propose,
            ai::commands::list_ollama_models,
            asr::commands::whisper_models,
            asr::commands::download_whisper_model,
            asr::commands::transcribe_project,
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
            export::cursor::pack_template::create_pack_template,
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
            let platform = std::sync::Arc::new(platform::current());
            app.manage(platform.clone());
            app.manage(session::project::commands::LaunchProject(
                session::project::commands::launch_project_from_argv(std::env::args()),
            ));
            let diag = crate::process::proc::init_ffmpeg(app.path().resource_dir().ok());
            let _ = std::fs::write(std::env::temp_dir().join("tcursor-ffmpeg.log"), diag);
            std::thread::spawn(crate::encode::ffmpeg_encoder::prewarm);
            std::thread::spawn(crate::export::preview::bg_thumbs::prewarm);
            if let Some(win) = app.get_webview_window("main") {
                const CAPTURE_EXCLUDE: bool = true;
                if CAPTURE_EXCLUDE {
                    if let Some(h) = crate::shell::window::handle(&win) {
                        let ok = platform.system.exclude_from_capture(h, true);
                        if ok {
                            println!("capture exclusion applied");
                        } else {
                            eprintln!(
                                "WARNING: capture exclusion FAILED - HUD may appear in recordings"
                            );
                        }
                    }
                } else {
                    println!("capture exclusion DISABLED (dev) - HUD is screenshottable");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
