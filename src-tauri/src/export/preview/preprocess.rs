// Pre-generate the editor's heavy preview media - proxy, filmstrip thumbnails, waveforms, mixed
// preview audio, and the edit.json seed - right after a recording stops, instead of lazily on
// editor open. Reuses the exact `ensure_*`/`load_or_seed` functions the editor's own lazy
// fallback calls (all `generate_once`-cached), so nothing here duplicates or re-runs their logic
// - the new behavior is running them eagerly, in sequence, with progress events, then marking the
// project preprocessed so `useEditorData` can skip its own lazy calls. This supersedes the old
// fire-and-forget `thumbs::prewarm` background spawn: the same sequence, now AWAITED by the
// frontend (with progress) instead of racing the editor's mount on a detached thread - that race
// was why the preview could still take a while to load right after Stop.
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::export::preview::preview_track::ensure_proxy;
use crate::export::preview::thumbs::{ensure_preview_audio, ensure_thumbs, ensure_waveform};
use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;

/// Proxy height generated during preprocessing - matches `Editor.tsx`'s initial `quality` state
/// and the frontend's `DEFAULT_PROXY_HEIGHT` (`src/lib/ipc.ts`), so a freshly preprocessed
/// project's default quality is always the one already sitting on disk.
pub const DEFAULT_PROXY_HEIGHT: u32 = 720;

/// Number of sequential steps `run` reports progress over - kept in one place so the emitted
/// percentages and the actual step count can never drift apart.
const STEP_COUNT: u32 = 6;

/// Kick off the full preprocessing pass for `folder` on a background thread and return
/// immediately - the frontend awaits completion via events, not this command's own promise.
/// Emits `preprocess-progress` (0..100) after each step, then exactly one of `preprocess-done`
/// (payload: the folder) or `preprocess-error` (payload: a message).
#[tauri::command]
pub fn preprocess_project(folder: String, app: AppHandle) {
    std::thread::spawn(move || {
        let app2 = app.clone();
        let folder2 = folder.clone();
        let result = run(&folder, move |pct| { let _ = app2.emit("preprocess-progress", pct); });
        match result {
            Ok(()) => { let _ = app.emit("preprocess-done", folder2); }
            Err(e) => { let _ = app.emit("preprocess-error", e); }
        }
    });
}

/// The sequence itself: proxy first (so the thumbnail pass reads the small proxy rather than the
/// raw capture - mirrors the old `prewarm` ordering), then filmstrip thumbs, system waveform, mic
/// waveform, mixed preview audio, and the `edit.json` seed - reporting progress after each. Only
/// on FULL success does it flip `manifest.preprocessed = true`; any failed step leaves it `false`
/// so a later open still falls back to the editor's own lazy `ensure_*` (same as an
/// un-preprocessed project - see the module doc comment).
fn run(folder: &str, on_progress: impl Fn(u32)) -> Result<(), String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };

    // The proxy is the ONE essential artifact - without it the editor preview is blank. Fail the
    // whole pass only if it fails. Everything after is best-effort (`let _ =`): a failure there
    // just leaves that single artifact to the editor's own lazy `ensure_*` fallback, instead of
    // aborting preprocessing entirely (leaving `preprocessed = false`) and forcing EVERYTHING -
    // including the already-built proxy - back onto the lazy path. That all-or-nothing `?` was why
    // one flaky step (e.g. a proxy transcode failure) blanked the whole preview.
    ensure_proxy(folder.to_string(), DEFAULT_PROXY_HEIGHT)?;
    on_progress(step_pct(1));
    let _ = ensure_thumbs(folder.to_string(), 16);
    on_progress(step_pct(2));
    let _ = ensure_waveform(folder.to_string(), "system".into());
    on_progress(step_pct(3));
    let _ = ensure_waveform(folder.to_string(), "mic".into());
    on_progress(step_pct(4));
    let _ = ensure_preview_audio(folder.to_string());
    on_progress(step_pct(5));
    crate::edit::seed::load_or_seed(&paths); // ensures edit.json exists on disk
    on_progress(step_pct(6));

    let mut manifest = ProjectManifest::load_or_default(&paths.manifest());
    manifest.preprocessed = true;
    manifest.save(&paths.manifest()).map_err(|e| e.to_string())
}

/// Pure: step `n` (1-based) of `STEP_COUNT` as a 0..100 percent - its own function so the
/// progression is unit-testable without touching ffmpeg.
fn step_pct(n: u32) -> u32 { (n * 100 / STEP_COUNT).min(100) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_pct_reaches_100_at_the_last_step() {
        assert_eq!(step_pct(STEP_COUNT), 100);
    }

    #[test]
    fn step_pct_is_monotonically_increasing() {
        let pcts: Vec<u32> = (1..=STEP_COUNT).map(step_pct).collect();
        for w in pcts.windows(2) { assert!(w[1] > w[0]); }
    }

    #[test]
    fn step_pct_never_exceeds_100() {
        for n in 0..=STEP_COUNT + 2 { assert!(step_pct(n) <= 100); }
    }
}
