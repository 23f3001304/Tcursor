use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use serde::Serialize;

use crate::domain::time::{Clock, SystemClock};
use crate::events::track::cursortracker::CursorTypeTracker;
use crate::events::model::ScreenInfo;
use crate::events::track::tracker::MouseTracker;
use crate::actions::keyboard::KeyboardTracker;
use crate::actions::matcher::arming_from_settings;
use crate::session::paths::ProjectPaths;
use crate::session::project::manifest::ProjectManifest;
use crate::session::project::recents;
use crate::session::record::recorder_threads::{save_inputs, spawn_mic_thread, spawn_system_thread};
use crate::session::record::video_sink::{start_video, VideoSink};

struct Running {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    video: VideoSink,
    mic_thread: Option<JoinHandle<()>>,
    system_thread: Option<JoinHandle<()>>,
    mouse: Option<MouseTracker>,
    keyboard: Option<KeyboardTracker>,
    cursor: Option<CursorTypeTracker>,
    events_path: PathBuf,
    actions_path: PathBuf,
    typing_path: PathBuf,
    cursor_path: PathBuf,
    screen: ScreenInfo,
    started_unix_ms: u64,
    events_ms: u64,
    mic_start: Arc<AtomicU64>,
    system_start: Arc<AtomicU64>,
    folder: String,
}

#[derive(Default)]
pub struct Recorder {
    inner: Mutex<Option<Running>>,
}

#[derive(Serialize)]
pub struct RecordingResult { pub folder: String, pub frames: u64 }

#[tauri::command]
pub fn start_recording(
    project_name: String,
    mic_id: Option<String>,
    target_id: Option<String>,
    system_audio: bool,
    game_mode: bool,
    recorder: tauri::State<'_, Recorder>,
) -> Result<String, String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() { return Err("already recording".into()); }

    let base = dirs_next::video_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor");
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
    let mic_start = Arc::new(AtomicU64::new(0));
    let system_start = Arc::new(AtomicU64::new(0));
    let events_ms = clock.now_ms();
    let mouse = Some(MouseTracker::start(8));
    let keyboard = Some(KeyboardTracker::start(arming_from_settings(&snap.hotkeys)));
    // Only track cursor shape when Enhanced (System/Hidden don't draw a synthetic cursor).
    let cursor = (snap.cursor.style == crate::settings::model::CursorStyle::Enhanced).then(CursorTypeTracker::start);
    let mic_thread = spawn_mic_thread(
        mic_id, paths.mic().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), mic_start.clone(),
    );
    let system_thread = spawn_system_thread(
        system_audio, paths.system().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), system_start.clone(),
    );

    // Slow part - the screen video pipeline - while the inputs above already run. GPU-native
    // (Media Foundation, no readback) by default; the compatibility toggle (game_mode) or a
    // GPU-encoder init failure falls back to the legacy ffmpeg path. Returns the captured (w, h).
    let video_path = paths.video().to_string_lossy().into_owned();
    let (video, w, h, origin_x, origin_y) = start_video(game_mode, clock.clone(), stop.clone(), paused.clone(),
        fps, snap.cursor.style.captures_os_cursor(), target_id.as_deref(), &video_path)?;
    println!("recording {w}x{h} @ {fps}fps (origin {origin_x},{origin_y})");

    let screen = ScreenInfo { w, h, origin_x, origin_y };
    let started_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    *guard = Some(Running {
        stop, paused, video,
        mic_thread, system_thread, mouse, keyboard, cursor,
        events_path: paths.events(), actions_path: paths.actions(),
        typing_path: paths.typing(), cursor_path: paths.cursor(),
        screen, started_unix_ms, events_ms, mic_start, system_start, folder: folder.clone(),
    });
    Ok(folder) // return the folder so the HUD can stream the webcam into it during recording
}

#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(r) => { r.paused.store(true, Ordering::SeqCst); Ok(()) }
        None => Err("not recording".into()),
    }
}

#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(r) => { r.paused.store(false, Ordering::SeqCst); Ok(()) }
        None => Err("not recording".into()),
    }
}

#[tauri::command]
pub fn stop_recording(recorder: tauri::State<'_, Recorder>) -> Result<RecordingResult, String> {
    let running = recorder.inner.lock().unwrap_or_else(|e| e.into_inner())
        .take().ok_or("not recording")?;

    // Signal the audio threads to stop; the video pipeline is stopped below.
    running.stop.store(true, Ordering::SeqCst);
    if let Some(t) = running.mic_thread { let _ = t.join(); }
    if let Some(t) = running.system_thread { let _ = t.join(); }

    // Save inputs before the video stop's ?-propagation so they survive a finalize error.
    save_inputs(running.mouse, running.keyboard, running.cursor,
        &running.events_path, &running.actions_path, &running.typing_path, &running.cursor_path,
        running.screen, running.started_unix_ms);

    // Stop + finalize the video pipeline (GPU: end capture + finish the MP4; ffmpeg: WM_QUIT + join).
    let (frames, frame_ts) = running.video.stop_and_collect()?;

    // Persist the real capture timeline so export can rebuild it (fps-agnostic).
    let pick = |c: &AtomicU64| { let v = c.load(Ordering::SeqCst); (v > 0).then_some(v) };
    let sync = crate::session::sync::SyncLog { frames: frame_ts, events_ms: running.events_ms,
        mic_ms: pick(&running.mic_start), system_ms: pick(&running.system_start) };
    let sync_path = std::path::Path::new(&running.folder).join("sync.json");
    if let Err(e) = sync.save(&sync_path) { eprintln!("sync.json save failed: {e}"); }

    // Write the .tcursor project manifest (best-effort - never fails the recording; the folder
    // is still a fully valid project without it, just not open-project-able by dialog until the
    // NEXT time it is written). preprocessed=false here: the frontend calls `preprocess_project`
    // right after this command resolves (shown as the HUD's "Saving..." progress) and that flips
    // it once the pass finishes. NOT done here as a detached background thread anymore - that
    // used to race the editor's mount (it could still be transcoding when the editor opened),
    // which is exactly the "preview still takes a while to load" lag; awaiting it with progress
    // in the caller fixes that.
    let paths = ProjectPaths { folder: PathBuf::from(&running.folder) };
    let manifest = ProjectManifest::new(running.screen.w, running.screen.h);
    if let Err(e) = manifest.save(&paths.manifest()) { eprintln!("project.tcursor save failed: {e}"); }
    recents::touch(&running.folder); // best-effort; also makes fresh recordings show up as "recent"

    Ok(RecordingResult { folder: running.folder, frames })
}
