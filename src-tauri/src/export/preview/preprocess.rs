// Pre-generate the editor's heavy preview media right after a recording stops, instead of lazily
// on editor open - the same `ensure_*_blocking`/`load_or_seed` functions the editor's own lazy
// `ensure_*` IPC commands call (all `generate_once`-cached), so nothing here duplicates their
// logic. What changed on 2026-09-14 (owner: "on long recordings it takes very long to come into
// the editor"): only the proxy is awaited. It is the one artifact the editor cannot open without,
// and on a long take it is most of the wait; the filmstrip, the two waveforms, the mixed preview
// audio and the edit.json seed follow on the same thread AFTER `preprocess-done`, and the editor
// (which requests all of them lazily anyway) simply gets each one as it lands - `generate_once`
// serialises a lazy request against the background pass, so no artifact is ever built twice.
// `essential` and `rest` call the `_blocking` variants directly rather than the
// `#[tauri::command] async fn` wrappers: this already runs on its own `std::thread`, off the
// Tokio runtime, so there is nothing to hop off of and no `.await` context to call them from.
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::export::preview::preview_track::ensure_proxy_with_progress;
use crate::export::preview::segments_audio::merge_mic_segments;
use crate::export::preview::segments_webcam::merge_webcam_segments;
use crate::export::preview::thumbs::{ensure_preview_audio_blocking, ensure_thumbs_blocking, ensure_waveform_blocking, FILMSTRIP_COUNT, FILMSTRIP_HEIGHT};
use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;
use crate::session::sync::SyncLog;

/// Proxy height generated during preprocessing - matches `Editor.tsx`'s initial `quality` state
/// and the frontend's `DEFAULT_PROXY_HEIGHT` (`src/lib/ipc.ts`), so a freshly preprocessed
/// project's default quality is always the one already sitting on disk.
pub const DEFAULT_PROXY_HEIGHT: u32 = 720;

/// Kick off preprocessing for `folder` on a background thread and return immediately - the
/// frontend awaits the editor-ready point via events, not this command's own promise. Emits
/// `preprocess-progress` (0..100, the proxy transcode's own progress) while the proxy builds, then
/// exactly one of `preprocess-done` (payload: the folder - the editor may open now) or
/// `preprocess-error` (payload: a message). The remaining artifacts keep building after `done`.
#[tauri::command]
pub fn preprocess_project(folder: String, app: AppHandle) {
    std::thread::spawn(move || {
        let app2 = app.clone();
        match essential(&folder, move |pct| { let _ = app2.emit("preprocess-progress", pct); }) {
            Ok(()) => { let _ = app.emit("preprocess-done", folder.clone()); rest(&folder); }
            Err(e) => { let _ = app.emit("preprocess-error", e); }
        }
    });
}

/// The one artifact the editor cannot open without: the default-quality proxy, with progress.
/// On success flips `manifest.preprocessed = true` - which `useEditorData` reads as "the default
/// proxy is on disk, point straight at it" - so the editor never shows the raw capture first. A
/// failure leaves it `false` and the editor's own lazy `ensure_proxy` gets another go.
fn essential(folder: &str, on_progress: impl Fn(u32)) -> Result<(), String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    // Mid-take source switches (2026-09-14): fold the extra mic and webcam segments back into
    // `mic.wav` and `webcam.webm` BEFORE anything reads them - `rest` builds the mic waveform and
    // the mixed preview audio from the one, the editor's stage plays the other. A take with no
    // switch lists nothing and this touches nothing. Neither ever fails the take: a merge that
    // errors leaves the first segment standing alone, which is what it was.
    let sync = SyncLog::load(&paths.sync()).unwrap_or_default();
    if let Err(e) = merge_mic_segments(&paths, &sync) { eprintln!("[PREPROCESS] mic merge: {e}"); }
    if let Err(e) = merge_webcam_segments(&paths, &sync) { eprintln!("[PREPROCESS] webcam merge: {e}"); }
    ensure_proxy_with_progress(folder.to_string(), DEFAULT_PROXY_HEIGHT, &on_progress)?;
    on_progress(100);
    let mut manifest = ProjectManifest::load_or_default(&paths.manifest());
    manifest.preprocessed = true;
    manifest.save(&paths.manifest()).map_err(|e| e.to_string())
}

/// Everything the editor can also fetch for itself, built after it has opened: filmstrip thumbs
/// (from the proxy, keyframes only), the system and mic waveforms, the mixed preview audio, and
/// the `edit.json` seed. Best-effort (`let _ =`): a failure here just leaves that one artifact to
/// the editor's lazy `ensure_*` fallback.
fn rest(folder: &str) {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    // The editor's own filmstrip request, verbatim (`thumbs::FILMSTRIP_*`) - a different count or
    // height here would fill a `thumbs_<count>_<height>` dir the editor never reads.
    let _ = ensure_thumbs_blocking(folder.to_string(), FILMSTRIP_COUNT, FILMSTRIP_HEIGHT);
    let _ = ensure_waveform_blocking(folder.to_string(), "system".into());
    let _ = ensure_waveform_blocking(folder.to_string(), "mic".into());
    let _ = ensure_preview_audio_blocking(folder.to_string());
    crate::edit::seed::load_or_seed(&paths); // ensures edit.json exists on disk
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
