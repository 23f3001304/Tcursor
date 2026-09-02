use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tauri::Emitter;

use crate::domain::time::{Clock, SystemClock};
use crate::events::track::cursortracker::CursorTypeTracker;
use crate::events::model::ScreenInfo;
use crate::events::track::tracker::MouseTracker;
use crate::actions::keyboard::KeyboardTracker;
use crate::actions::matcher::arming_from_settings;
use crate::session::paths::ProjectPaths;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::recorder_threads::{spawn_mic_thread, spawn_system_thread};
use crate::session::record::video_sink::{start_video, VideoSink, VideoStart};
use crate::session::record::Notify;

pub(super) struct Running {
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub paused_totals: Arc<PauseTotals>,
    pub clock: Arc<dyn Clock>,
    pub video: VideoSink,
    pub mic_thread: Option<JoinHandle<()>>,
    pub system_thread: Option<JoinHandle<()>>,
    pub mouse: Option<MouseTracker>,
    pub keyboard: Option<KeyboardTracker>,
    pub cursor: Option<CursorTypeTracker>,
    pub events_path: PathBuf,
    pub actions_path: PathBuf,
    pub typing_path: PathBuf,
    pub cursor_path: PathBuf,
    pub screen: ScreenInfo,
    pub started_unix_ms: u64,
    pub events_ms: u64,
    pub mic_start: Arc<AtomicU64>,
    pub system_start: Arc<AtomicU64>,
    pub folder: String,
}

#[derive(Default)]
pub struct Recorder {
    pub(super) inner: Mutex<Option<Running>>,
    /// Set while `stop_recording` is tearing the session down. `Running` has already been taken
    /// out of `inner` by then, so `inner.is_some()` alone would let a start racing the tail of a
    /// stop through - and the new `MouseTracker` would overwrite the process-global hook sink
    /// before the old stop reads it (finished take gets the new empty collector; new take
    /// records zero mouse events, so no cursor, no click FX, no auto-zoom). Read and written
    /// only while holding `inner`'s lock, so it is effectively part of that guarded state.
    pub(super) stopping: AtomicBool,
}

impl Recorder {
    /// True while a take is actively recording (`inner` is `Some`) - i.e. nobody has started
    /// stopping it yet. Used by `lib.rs`'s `CloseRequested` guard to decide whether IT is the one
    /// that must run `stop_recording`, versus someone else already owning the stop.
    pub fn is_recording(&self) -> bool {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// True while a take is recording OR its stop is still finalizing - `is_recording()` OR
    /// `stopping`. *Why both:* `stop_blocking` takes `Running` out of `inner` (making
    /// `is_recording()` false) well before the finalize it then runs (joining audio threads,
    /// writing the video's `moov` atom, `sync.json`, the manifest) actually completes - `stopping`
    /// covers exactly that window. The `CloseRequested` guard in `lib.rs` uses THIS (not just
    /// `is_recording()`) so a close request landing in that gap can't slip through and let the
    /// process exit mid-finalize; see `close_guard::finish_and_close`.
    pub fn is_busy(&self) -> bool {
        self.is_recording() || self.stopping.load(Ordering::SeqCst)
    }
}

/// A `Notify` that forwards its reason to the frontend as `event`. Recording threads hold these
/// instead of an `AppHandle`, so nothing below this file needs to know about Tauri.
fn emitter(app: &tauri::AppHandle, event: &'static str) -> Notify {
    let app = app.clone();
    Arc::new(move |reason: &str| { let _ = app.emit(event, reason.to_string()); })
}

#[tauri::command]
pub fn start_recording(
    project_name: String,
    mic_id: Option<String>,
    target_id: Option<String>,
    system_audio: bool,
    game_mode: bool,
    recorder: tauri::State<'_, Recorder>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() || recorder.stopping.load(Ordering::SeqCst) { return Err("already recording".into()); }

    let base = dirs_next::video_dir().unwrap_or_else(std::env::temp_dir).join("TCursor");
    let paths = ProjectPaths::new(&base, &project_name);
    paths.ensure().map_err(|e| format!("prepare folder: {e}"))?;
    // Snapshot the active settings so the export reproduces this recording exactly.
    let snap = crate::settings::store::load();
    if let Ok(bytes) = serde_json::to_vec(&snap) {
        let _ = std::fs::write(paths.settings(), bytes);
    }
    let folder = paths.folder.to_string_lossy().into_owned();

    let fps = crate::win::sys::display::primary_refresh_hz().min(60);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());

    // Start every capture input (mouse, keyboard, mic, system audio) BEFORE the screen video
    // pipeline. Building the encoder can take a moment (first ffmpeg scan / MF setup); anything
    // spawned after it would miss that gap and land late in the export. These run concurrently
    // with the slow setup, so they start ~when the screen does.
    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let paused_totals = Arc::new(PauseTotals::new());
    let mic_start = Arc::new(AtomicU64::new(0));
    let system_start = Arc::new(AtomicU64::new(0));
    let events_ms = clock.now_ms();
    let warn = emitter(&app, "record-warning");
    let mouse = Some(MouseTracker::start(8, paused_totals.clone()));
    let keyboard = Some(KeyboardTracker::start(arming_from_settings(&snap.hotkeys), paused_totals.clone()));
    // Only track cursor shape when Enhanced (System/Hidden don't draw a synthetic cursor).
    let cursor = (snap.cursor.style == crate::settings::model::CursorStyle::Enhanced).then(|| CursorTypeTracker::start(paused_totals.clone()));
    let mic_thread = spawn_mic_thread(
        mic_id, paths.mic().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), mic_start.clone(), warn.clone(),
    );
    let system_thread = spawn_system_thread(
        system_audio, paths.system().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), system_start.clone(), warn,
    );

    // Slow part - the screen video pipeline - while the inputs above already run. GPU-native
    // (Media Foundation, no readback) by default; the compatibility toggle (game_mode) or a
    // GPU-encoder init failure falls back to the legacy ffmpeg path. Returns the captured (w, h).
    let video_path = paths.video().to_string_lossy().into_owned();
    let cfg = VideoStart {
        legacy: game_mode, clock: clock.clone(), stop: stop.clone(), paused: paused.clone(),
        totals: paused_totals.clone(), ended: emitter(&app, "record-ended-early"),
        fps, with_cursor: snap.cursor.style.captures_os_cursor(),
    };
    let (video, w, h, origin_x, origin_y) = match start_video(cfg, target_id.as_deref(), &video_path) {
        Ok(v) => v,
        Err(e) => {
            // start_video failed after the mic/system-audio threads were already spawned
            // (see comment above - they intentionally run concurrently with the slow
            // encoder init). Without this, `stop` is never set, so those threads' 50ms
            // poll loops never exit: a leaked thread plus a mic/loopback device left open
            // and its WAV never finalized. mouse/keyboard/cursor need no equivalent
            // handling here - they're plain locals (never moved into `Running`), so their
            // Drop impls already stop the underlying hooks when this function returns.
            stop.store(true, Ordering::SeqCst);
            if let Some(t) = mic_thread { let _ = t.join(); }
            if let Some(t) = system_thread { let _ = t.join(); }
            return Err(e);
        }
    };
    println!("recording {w}x{h} @ {fps}fps (origin {origin_x},{origin_y})");

    let screen = ScreenInfo { w, h, origin_x, origin_y };
    let started_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

    *guard = Some(Running {
        stop, paused, paused_totals, clock, video,
        mic_thread, system_thread, mouse, keyboard, cursor,
        events_path: paths.events(), actions_path: paths.actions(),
        typing_path: paths.typing(), cursor_path: paths.cursor(),
        screen, started_unix_ms, events_ms, mic_start, system_start, folder: folder.clone(),
    });
    crate::win::sys::brand_icon::set_recording(&app, true); // brand flair only - never fails the recording
    Ok(folder) // return the folder so the HUD can stream the webcam into it during recording
}

#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        // Ledger stamped under this same lock, from this same clock, beside the flag flip -
        // the exact span every input tracker's `elapsed_paused` subtracts at its stamp site,
        // and (since C1) the span both capture paths remove from the video's own timeline.
        Some(r) => { r.paused_totals.pause(r.clock.now_ms()); r.paused.store(true, Ordering::SeqCst); Ok(()) }
        None => Err("not recording".into()),
    }
}

#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(r) => { r.paused_totals.resume(r.clock.now_ms()); r.paused.store(false, Ordering::SeqCst); Ok(()) }
        None => Err("not recording".into()),
    }
}

#[cfg(test)]
#[path = "recorder_tests.rs"]
mod tests;
