use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use serde::Serialize;

use crate::capture::frame_source::FrameSource;
use crate::capture::windows_capture::WgcFrameSource;
use crate::domain::time::{Clock, SystemClock};
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::events::model::{EventLog, ScreenInfo};
use crate::events::tracker::MouseTracker;
use crate::actions::keyboard::KeyboardTracker;
use crate::actions::matcher::arming_from_settings;
use crate::actions::model::ActionLog;
use crate::session::paths::ProjectPaths;
use crate::session::recorder_threads::{spawn_mic_thread, spawn_system_thread};
use crate::session::recording_session::RecordingSession;

struct Running {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    video_halt: Arc<AtomicBool>,
    video_stopper: Option<Box<dyn FnOnce() + Send>>,
    video_thread: JoinHandle<std::io::Result<(u64, Vec<u64>)>>,
    mic_thread: Option<JoinHandle<()>>,
    system_thread: Option<JoinHandle<()>>,
    mouse: Option<MouseTracker>,
    keyboard: Option<KeyboardTracker>,
    events_path: PathBuf,
    actions_path: PathBuf,
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
    system_audio: bool,
    recorder: tauri::State<'_, Recorder>,
) -> Result<(), String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() { return Err("already recording".into()); }

    let base = dirs_next::video_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor");
    let paths = ProjectPaths::new(&base, &project_name);
    paths.ensure().map_err(|e| e.to_string())?;
    // Snapshot the active settings so the export reproduces this recording exactly.
    let snap = crate::settings::store::load();
    if let Ok(bytes) = serde_json::to_vec(&snap) {
        let _ = std::fs::write(paths.settings(), bytes);
    }
    let folder = paths.folder.to_string_lossy().into_owned();

    let fps = crate::win::display::primary_refresh_hz().min(60);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let mut source = WgcFrameSource::for_primary_display(clock.clone(), fps).map_err(|e| e.to_string())?;
    let (w, h) = source.dimensions();

    // Start every capture input (mouse, keyboard, mic, system audio) BEFORE the
    // ffmpeg sink. Creating the sink can take seconds on the first recording (the
    // bundled ffmpeg is scanned / encoders are probed); anything spawned after it
    // would miss that long and land seconds late in the export. These threads run
    // concurrently with the slow sink setup, so they start ~when the screen does.
    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let mic_start = Arc::new(AtomicU64::new(0));
    let system_start = Arc::new(AtomicU64::new(0));
    let events_ms = clock.now_ms();
    let mouse = Some(MouseTracker::start(8));
    let keyboard = Some(KeyboardTracker::start(arming_from_settings(&snap.hotkeys)));
    let mic_thread = spawn_mic_thread(
        mic_id, paths.mic().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), mic_start.clone(),
    );
    let system_thread = spawn_system_thread(
        system_audio, paths.system().to_string_lossy().into_owned(),
        stop.clone(), paused.clone(), clock.clone(), system_start.clone(),
    );

    // Now the slow part — the screen encoder — while the inputs above already run.
    let video_path = paths.video().to_string_lossy().into_owned();
    let sink = FfmpegFrameSink::new(&video_path, w, h, fps).map_err(|e| e.to_string())?;
    println!("recording {w}x{h} @ {fps}fps");

    let video_halt = source.halt_handle();
    let video_stopper = source.take_stopper();
    let actions_path = paths.actions();
    let screen = ScreenInfo { w, h, origin_x: 0, origin_y: 0 };
    let started_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let events_path = paths.events();

    let video_stop = stop.clone();
    let video_paused = paused.clone();
    let video_thread = std::thread::Builder::new()
        .name("video".into())
        .spawn(move || {
            let mut session = RecordingSession::new(Box::new(source), Box::new(sink));
            session.run(&video_stop, &video_paused);
            let frame_ts = session.frame_timestamps().to_vec();
            let n = session.stop_and_finalize()?;
            Ok((n, frame_ts))
        })
        .map_err(|e| e.to_string())?;

    *guard = Some(Running {
        stop, paused, video_halt, video_stopper, video_thread,
        mic_thread, system_thread, mouse, keyboard, events_path, actions_path, screen, started_unix_ms,
        events_ms, mic_start, system_start, folder,
    });
    Ok(())
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

    // Stop WGC capture promptly, then signal all threads via shared flag.
    running.video_halt.store(true, Ordering::SeqCst);
    if let Some(stopper) = running.video_stopper { stopper(); }
    running.stop.store(true, Ordering::SeqCst);

    if let Some(t) = running.mic_thread { let _ = t.join(); }
    if let Some(t) = running.system_thread { let _ = t.join(); }

    // Save events before the video join's ?-propagation so they survive a finalize error.
    if let Some(tracker) = running.mouse {
        let events = tracker.stop();
        let log = EventLog { started_unix_ms: running.started_unix_ms, screen: running.screen, events };
        if let Err(e) = log.save(&running.events_path) { eprintln!("events.json save failed: {e}"); }
    }

    if let Some(kb) = running.keyboard {
        let log = ActionLog { actions: kb.stop() };
        if let Err(e) = log.save(&running.actions_path) { eprintln!("actions.json save failed: {e}"); }
    }

    let (frames, frame_ts) = running.video_thread
        .join().map_err(|_| "video thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    // Persist the real capture timeline so export can rebuild it (fps-agnostic).
    let pick = |c: &AtomicU64| { let v = c.load(Ordering::SeqCst); (v > 0).then_some(v) };
    let sync = crate::session::sync::SyncLog {
        frames: frame_ts,
        events_ms: running.events_ms,
        mic_ms: pick(&running.mic_start),
        system_ms: pick(&running.system_start),
    };
    let sync_path = std::path::Path::new(&running.folder).join("sync.json");
    if let Err(e) = sync.save(&sync_path) { eprintln!("sync.json save failed: {e}"); }

    Ok(RecordingResult { folder: running.folder, frames })
}
