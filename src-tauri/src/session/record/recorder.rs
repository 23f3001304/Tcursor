use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::actions::matcher::arming_from_settings;
use crate::domain::time::{Clock, SystemClock};
use crate::events::model::ScreenInfo;
use crate::platform::Platform;
use crate::ports::capture::{CaptureRequest, TargetId, VideoSink};
use crate::ports::input::{CursorShapePort, HotkeyPort, PointerPort};
use crate::session::paths::ProjectPaths;
use crate::session::record::emit::TakeHooks;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::recorder_threads::{spawn_mic_thread, spawn_system_thread};
use crate::session::record::segments::{self, SharedSegments};

pub(super) struct Running {
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub paused_totals: Arc<PauseTotals>,
    pub clock: Arc<dyn Clock>,
    pub video: Box<dyn VideoSink>,
    pub mic_thread: Option<JoinHandle<()>>,
    pub mic_stop: Arc<AtomicBool>,
    pub system_thread: Option<JoinHandle<()>>,
    pub mouse: Box<dyn PointerPort>,
    pub keyboard: Box<dyn HotkeyPort>,
    pub cursor: Box<dyn CursorShapePort>,
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
    pub segments: SharedSegments,
}

#[derive(Default)]
pub struct Recorder {
    pub(super) inner: Mutex<Option<Running>>,
    pub(super) stopping: AtomicBool,
}

impl Recorder {
    pub fn is_recording(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    pub fn is_busy(&self) -> bool {
        self.is_recording() || self.stopping.load(Ordering::SeqCst)
    }
}

pub struct TakeSpec {
    pub base: PathBuf,
    pub project_name: String,
    pub mic_id: Option<String>,
    pub target_id: Option<String>,
    pub system_audio: bool,
    pub game_mode: bool,
}

pub fn start_take(
    spec: TakeSpec,
    hooks: TakeHooks,
    platform: &Platform,
    recorder: &Recorder,
) -> Result<String, String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() || recorder.stopping.load(Ordering::SeqCst) {
        return Err("already recording".into());
    }

    let paths = ProjectPaths::new(&spec.base, &spec.project_name);
    paths.ensure().map_err(|e| format!("prepare folder: {e}"))?;
    let snap = crate::settings::store::load();
    if let Ok(bytes) = serde_json::to_vec(&snap) {
        let _ = std::fs::write(paths.settings(), bytes);
    }
    let folder = paths.folder.to_string_lossy().into_owned();

    let fps = platform.system.primary_refresh_hz().min(60);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());

    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let paused_totals = Arc::new(PauseTotals::new());
    let mic_start = Arc::new(AtomicU64::new(0));
    let system_start = Arc::new(AtomicU64::new(0));
    let events_ms = clock.now_ms();
    let mouse = platform.input.pointer(8, paused_totals.clone());
    let keyboard = platform
        .input
        .hotkeys(arming_from_settings(&snap.hotkeys), paused_totals.clone());
    let cursor = platform.input.cursor_shapes(paused_totals.clone());
    let mic_stop = Arc::new(AtomicBool::new(false));
    let mic_thread = spawn_mic_thread(
        spec.mic_id,
        paths.mic().to_string_lossy().into_owned(),
        mic_stop.clone(),
        paused.clone(),
        clock.clone(),
        mic_start.clone(),
        hooks.warn.clone(),
        hooks.mic_level,
    );
    let system_thread = spec
        .system_audio
        .then(|| {
            spawn_system_thread(
                platform.audio.loopback_device(),
                paths.system().to_string_lossy().into_owned(),
                stop.clone(),
                paused.clone(),
                clock.clone(),
                system_start.clone(),
                hooks.warn,
                hooks.system_level,
            )
        })
        .flatten();

    let req = CaptureRequest {
        target: TargetId::from_arg(spec.target_id.as_deref()),
        output: paths.video(),
        fps,
        with_cursor: false,
        prefer_compatibility: spec.game_mode,
        clock: clock.clone(),
        stop: stop.clone(),
        paused: paused.clone(),
        totals: paused_totals.clone(),
        ended: hooks.ended,
    };
    let (video, geometry) = match platform.capture.start(req) {
        Ok(v) => v,
        Err(e) => {
            stop.store(true, Ordering::SeqCst);
            if let Some(t) = mic_thread {
                let _ = t.join();
            }
            if let Some(t) = system_thread {
                let _ = t.join();
            }
            return Err(e);
        }
    };
    println!(
        "recording {}x{} @ {fps}fps (origin {},{})",
        geometry.w, geometry.h, geometry.origin_x, geometry.origin_y
    );

    let screen = ScreenInfo {
        w: geometry.w,
        h: geometry.h,
        origin_x: geometry.origin_x,
        origin_y: geometry.origin_y,
    };
    let started_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    *guard = Some(Running {
        stop,
        paused,
        paused_totals,
        clock,
        video,
        mic_thread,
        mic_stop,
        system_thread,
        mouse,
        keyboard,
        cursor,
        events_path: paths.events(),
        actions_path: paths.actions(),
        typing_path: paths.typing(),
        cursor_path: paths.cursor(),
        screen,
        started_unix_ms,
        events_ms,
        mic_start,
        system_start,
        folder: folder.clone(),
        segments: segments::shared(),
    });
    Ok(folder)
}

pub fn pause_take(recorder: &Recorder) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(r) => {
            r.paused_totals.pause(r.clock.now_ms());
            r.paused.store(true, Ordering::SeqCst);
            Ok(())
        }
        None => Err("not recording".into()),
    }
}

pub fn resume_take(recorder: &Recorder) -> Result<(), String> {
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(r) => {
            r.paused_totals.resume(r.clock.now_ms());
            r.paused.store(false, Ordering::SeqCst);
            Ok(())
        }
        None => Err("not recording".into()),
    }
}

#[tauri::command]
pub fn start_recording(
    project_name: String,
    mic_id: Option<String>,
    target_id: Option<String>,
    system_audio: bool,
    game_mode: bool,
    recorder: tauri::State<'_, Recorder>,
    platform: tauri::State<'_, Arc<Platform>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let spec = TakeSpec {
        base: dirs_next::video_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("TCursor"),
        project_name,
        mic_id,
        target_id,
        system_audio,
        game_mode,
    };
    let folder = start_take(spec, TakeHooks::from_app(&app), &platform, &recorder)?;
    crate::shell::brand_icon::set_recording(&app, true);
    Ok(folder)
}

#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    pause_take(&recorder)
}

#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    resume_take(&recorder)
}

#[cfg(test)]
#[path = "recorder_tests.rs"]
mod tests;
