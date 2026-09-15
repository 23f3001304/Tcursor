use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::export::preview::preview_track::ensure_proxy_with_progress;
use crate::export::preview::segments_audio::merge_mic_segments;
use crate::export::preview::segments_webcam::merge_webcam_segments;
use crate::export::preview::thumbs::{
    ensure_preview_audio_blocking, ensure_thumbs_blocking, ensure_waveform_blocking,
    FILMSTRIP_COUNT, FILMSTRIP_HEIGHT,
};
use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;
use crate::session::sync::SyncLog;

pub const DEFAULT_PROXY_HEIGHT: u32 = 720;

#[tauri::command]
pub fn preprocess_project(folder: String, app: AppHandle) {
    std::thread::spawn(move || {
        let app2 = app.clone();
        match essential(&folder, move |pct| {
            let _ = app2.emit("preprocess-progress", pct);
        }) {
            Ok(()) => {
                let _ = app.emit("preprocess-done", folder.clone());
                rest(&folder);
            }
            Err(e) => {
                let _ = app.emit("preprocess-error", e);
            }
        }
    });
}

fn essential(folder: &str, on_progress: impl Fn(u32)) -> Result<(), String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(folder),
    };
    let sync = SyncLog::load(&paths.sync()).unwrap_or_default();
    if let Err(e) = merge_mic_segments(&paths, &sync) {
        eprintln!("[PREPROCESS] mic merge: {e}");
    }
    if let Err(e) = merge_webcam_segments(&paths, &sync) {
        eprintln!("[PREPROCESS] webcam merge: {e}");
    }
    ensure_proxy_with_progress(folder.to_string(), DEFAULT_PROXY_HEIGHT, &on_progress)?;
    on_progress(100);
    let mut manifest = ProjectManifest::load_or_default(&paths.manifest());
    manifest.preprocessed = true;
    manifest.save(&paths.manifest()).map_err(|e| e.to_string())
}

fn rest(folder: &str) {
    let paths = ProjectPaths {
        folder: PathBuf::from(folder),
    };
    let _ = ensure_thumbs_blocking(folder.to_string(), FILMSTRIP_COUNT, FILMSTRIP_HEIGHT);
    let _ = ensure_waveform_blocking(folder.to_string(), "system".into());
    let _ = ensure_waveform_blocking(folder.to_string(), "mic".into());
    let _ = ensure_preview_audio_blocking(folder.to_string());
    crate::edit::seed::load_or_seed(&paths);
}

#[cfg(test)]
mod tests {
    use crate::export::preview::preview_track::proxy_pct;

    #[test]
    fn a_progress_line_maps_output_time_onto_the_real_duration() {
        assert_eq!(proxy_pct("out_time_us=30000000", 120.0), Some(25));
        assert_eq!(proxy_pct("out_time_us=0", 120.0), Some(0));
    }

    #[test]
    fn the_transcode_never_reports_done_itself_and_ignores_the_rest_of_the_stream() {
        assert_eq!(proxy_pct("out_time_us=130000000", 120.0), Some(99));
        assert_eq!(proxy_pct("frame=1200", 120.0), None);
        assert_eq!(proxy_pct("progress=continue", 120.0), None);
        assert_eq!(proxy_pct("out_time_us=nope", 120.0), None);
    }
}
