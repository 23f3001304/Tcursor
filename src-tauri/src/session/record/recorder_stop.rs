//! The Stop half of the recorder command surface, split from `recorder.rs` (which owns the
//! session state and Start/Pause/Resume) so both stay under the line cap.
use std::sync::atomic::{AtomicU64, Ordering};
use serde::Serialize;
use tauri::Manager;

use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_threads::{save_inputs, save_session_files};

#[derive(Serialize)]
pub struct RecordingResult { pub folder: String, pub frames: u64 }

/// Clears `Recorder::stopping` when the teardown leaves scope, however it leaves. A plain store
/// at the end of `stop_blocking` would leave the flag stuck on after a panic anywhere in the
/// teardown, and a stuck flag means every later `start_recording` returns "already recording" -
/// the Record button dead for the rest of the session.
struct StoppingGuard<'a>(&'a Recorder);

impl Drop for StoppingGuard<'_> {
    fn drop(&mut self) {
        // Under the lock so "a start either sees stopping, or sees a fully torn-down recorder"
        // is an ordering guarantee rather than an accident of timing.
        let _guard = self.0.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.0.stopping.store(false, Ordering::SeqCst);
    }
}

/// `async` + `spawn_blocking`: this command joins the mic/system-audio threads, gzips and writes
/// the whole input log, and then joins the video pipeline - which in both paths waits for the
/// encoder to write the `moov` atom of a potentially multi-GB MP4. As a sync command all of that
/// ran on the main thread, so Stop froze the HUD (no repaint, no spinner motion) for the entire
/// finalize. Same conversion `ai::commands`, `thumbs.rs` and `preview_track.rs` already had.
#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<RecordingResult, String> {
    tauri::async_runtime::spawn_blocking(move || stop_blocking(&app))
        .await
        .map_err(|e| e.to_string())?
}

fn stop_blocking(app: &tauri::AppHandle) -> Result<RecordingResult, String> {
    let recorder = app.state::<Recorder>();
    // Take the session AND claim the stopping flag under one lock acquisition. The teardown
    // below runs outside the lock (it joins threads and finalizes a huge MP4 - holding the
    // recorder lock across that would block Pause/Resume on the main thread), so the flag is
    // what keeps a concurrent Start out until the old `MouseTracker` has released the global
    // hook sink; see `Recorder::stopping`.
    let running = {
        let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
        let running = guard.take().ok_or("not recording")?;
        recorder.stopping.store(true, Ordering::SeqCst);
        running
    };
    let _stopping = StoppingGuard(&recorder);
    crate::win::sys::brand_icon::set_recording(app, false); // brand flair only - never fails the stop

    // Signal the audio threads to stop; the video pipeline is stopped below.
    running.stop.store(true, Ordering::SeqCst);
    if let Some(t) = running.mic_thread { let _ = t.join(); }
    if let Some(t) = running.system_thread { let _ = t.join(); }

    // Inputs first: these are cheap and must survive a video finalize failure.
    save_inputs(running.mouse, running.keyboard, running.cursor,
        &running.events_path, &running.actions_path, &running.typing_path, &running.cursor_path,
        running.screen, running.started_unix_ms);

    // Stop + finalize the video pipeline (GPU: end capture + finish the MP4; ffmpeg: WM_QUIT +
    // join). A finalize failure comes back as `stopped.error` INSTEAD of short-circuiting: the
    // frames it did record are still on disk, so sync.json, the `.tcursor` manifest and the
    // recents entry are written from them either way. Propagating first (as this used to) left
    // a folder with no manifest - which `open_project`'s `*.tcursor` filter cannot even select -
    // and no sync.json, so the export would have synthesised a timeline and mis-placed the mic.
    let stopped = running.video.stop_and_collect();
    let pick = |c: &AtomicU64| { let v = c.load(Ordering::SeqCst); (v > 0).then_some(v) };
    save_session_files(&running.folder, stopped.frame_ts, running.events_ms,
        pick(&running.mic_start), pick(&running.system_start), running.screen);

    // `_stopping` drops here, releasing the guard now that the old take is fully detached from
    // the process-global input hooks.
    match stopped.error {
        Some(e) => Err(e), // the project folder is saved; the HUD still shows what went wrong
        None => Ok(RecordingResult { folder: running.folder, frames: stopped.frames }),
    }
}
