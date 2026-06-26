use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use serde::Serialize;

use crate::capture::frame_source::FrameSource;
use crate::capture::windows_capture::WgcFrameSource;
use crate::domain::time::SystemClock;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::events::model::{EventLog, ScreenInfo};
use crate::events::tracker::MouseTracker;
use crate::session::paths::ProjectPaths;
use crate::session::recorder_threads::{spawn_mic_thread, spawn_system_thread};
use crate::session::recording_session::RecordingSession;

#[derive(Serialize)]
pub struct DisplayInfo { pub id: u32, pub label: String }

#[derive(Serialize)]
pub struct AudioInfo { pub id: String, pub label: String }

#[derive(Serialize)]
pub struct RecordingResult { pub folder: String, pub frames: u64 }

struct Running {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    video_halt: Arc<AtomicBool>,
    video_stopper: Option<Box<dyn FnOnce() + Send>>,
    video_thread: JoinHandle<std::io::Result<u64>>,
    mic_thread: Option<JoinHandle<()>>,
    system_thread: Option<JoinHandle<()>>,
    mouse: Option<MouseTracker>,
    events_path: PathBuf,
    screen: ScreenInfo,
    started_unix_ms: u64,
    folder: String,
}

#[derive(Default)]
pub struct Recorder {
    inner: Mutex<Option<Running>>,
}

#[tauri::command]
pub fn list_displays() -> Vec<DisplayInfo> {
    vec![DisplayInfo { id: 0, label: "Primary Display".into() }]
}

#[tauri::command]
pub fn list_audio_inputs() -> Vec<AudioInfo> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| it.filter_map(|d| {
            let name = d.name().ok()?;
            Some(AudioInfo { id: name.clone(), label: name })
        }).collect())
        .unwrap_or_default()
}

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
        .join("CursorZoom");
    let paths = ProjectPaths::new(&base, &project_name);
    paths.ensure().map_err(|e| e.to_string())?;
    let folder = paths.folder.to_string_lossy().into_owned();

    let fps = crate::win::display::primary_refresh_hz().min(60);
    let clock = Arc::new(SystemClock::new());
    let mut source = WgcFrameSource::for_primary_display(clock, fps).map_err(|e| e.to_string())?;
    let (w, h) = source.dimensions();
    let video_path = paths.video().to_string_lossy().into_owned();
    let sink = FfmpegFrameSink::new(&video_path, w, h, fps).map_err(|e| e.to_string())?;
    println!("recording {w}x{h} @ {fps}fps");

    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));

    let video_halt = source.halt_handle();
    let video_stopper = source.take_stopper();

    let mouse = Some(MouseTracker::start(8));
    let screen = ScreenInfo { w, h, origin_x: 0, origin_y: 0 };
    let started_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let events_path = paths.events();

    let mic_thread = spawn_mic_thread(
        mic_id,
        paths.mic().to_string_lossy().into_owned(),
        stop.clone(),
        paused.clone(),
    );
    let system_thread = spawn_system_thread(
        system_audio,
        paths.system().to_string_lossy().into_owned(),
        stop.clone(),
        paused.clone(),
    );

    let video_stop = stop.clone();
    let video_paused = paused.clone();
    let video_thread = std::thread::Builder::new()
        .name("video".into())
        .spawn(move || {
            let mut session = RecordingSession::new(Box::new(source), Box::new(sink));
            session.run(&video_stop, &video_paused);
            session.stop_and_finalize()
        })
        .map_err(|e| e.to_string())?;

    *guard = Some(Running {
        stop, paused, video_halt, video_stopper, video_thread,
        mic_thread, system_thread, mouse, events_path, screen, started_unix_ms, folder,
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

    let frames = running.video_thread
        .join().map_err(|_| "video thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    Ok(RecordingResult { folder: running.folder, frames })
}

#[tauri::command]
pub fn save_webcam(folder: String, bytes: Vec<u8>) -> Result<(), String> {
    let path = std::path::Path::new(&folder).join("webcam.webm");
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_project(folder: String, app: tauri::AppHandle) -> Result<(), String> {
    crate::export::run::run_export(app, folder);
    Ok(())
}
