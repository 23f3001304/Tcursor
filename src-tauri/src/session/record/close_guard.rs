//! The `CloseRequested` safety net (task-6 brief, ruling R6): if the window tries to close while
//! `Recorder::is_busy()`, `lib.rs`'s `on_window_event` handler calls `prevent_close()` and hands
//! off to `finish_and_close` here instead of ever letting the process exit mid-take.
use tauri::Manager;

use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_stop::stop_recording;

/// Finishes the take (if nobody has already claimed the stop) and then closes the main window.
/// Spawned once per close attempt from `lib.rs`.
///
/// *Why this does not depend on the frontend.* This is the fallback for the OS close button,
/// Alt+F4, "Close window" from the taskbar, and a wedged renderer - all of which reach
/// `CloseRequested` with no cooperating JS at all (a hung renderer still lets the OS deliver
/// `WM_CLOSE` to the native window). Depending on the HUD's own JS-side graceful stop
/// (`useRecordingFlow.stopForClose`) for correctness here would reintroduce the exact bug this
/// guard exists to close. The trade-off accepted: a webcam `MediaRecorder`'s last buffered chunk
/// lives only in the renderer and cannot be flushed from here, so it may be missing when this
/// path (rather than the HUD's own) is what performs the stop. The primary artifact - screen,
/// mic, system audio, `sync.json`, the manifest - is what must never be lost, and IS finalized by
/// `stop_recording` regardless of the frontend's state.
pub async fn finish_and_close(app: tauri::AppHandle) {
    let recorder = app.state::<Recorder>();
    if recorder.is_recording() {
        // Nobody has claimed the stop yet - this IS the stop.
        let _ = stop_recording(app.clone()).await;
    } else {
        // Someone else (most likely the HUD's own graceful-close path) already took `Running` out
        // of `inner` and is finalizing it right now - wait for THAT instead of racing a second,
        // redundant `stop_recording` call (which would just return "not recording" immediately,
        // closing the window before the real finalize underneath it has actually finished).
        let app2 = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let recorder = app2.state::<Recorder>();
            while recorder.is_busy() {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        })
        .await;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.close();
    }
}
