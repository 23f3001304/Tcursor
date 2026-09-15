use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

use crate::ports::capture::VideoSink;
use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_threads::{save_inputs, save_session_files};

#[derive(Serialize)]
pub struct RecordingResult {
    pub folder: String,
    pub frames: u64,
}

struct StoppingGuard<'a>(&'a Recorder);

impl Drop for StoppingGuard<'_> {
    fn drop(&mut self) {
        let _guard = self.0.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.0.stopping.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<RecordingResult, String> {
    tauri::async_runtime::spawn_blocking(move || stop_blocking(&app))
        .await
        .map_err(|e| e.to_string())?
}

fn stop_blocking(app: &tauri::AppHandle) -> Result<RecordingResult, String> {
    let recorder = app.state::<Recorder>();
    stop_take(&recorder, &|| {
        crate::shell::brand_icon::set_recording(app, false)
    })
}

pub fn stop_take(recorder: &Recorder, claimed: &dyn Fn()) -> Result<RecordingResult, String> {
    let running = {
        let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
        let running = guard.take().ok_or("not recording")?;
        recorder.stopping.store(true, Ordering::SeqCst);
        running
    };
    let _stopping = StoppingGuard(recorder);
    claimed();

    running.stop.store(true, Ordering::SeqCst);
    running.mic_stop.store(true, Ordering::SeqCst);
    if let Some(t) = running.mic_thread {
        let _ = t.join();
    }
    if let Some(t) = running.system_thread {
        let _ = t.join();
    }

    let paths = crate::session::paths::ProjectPaths {
        folder: std::path::PathBuf::from(&running.folder),
    };
    save_inputs(
        running.mouse,
        running.keyboard,
        running.cursor,
        &running.events_path,
        &running.actions_path,
        &running.typing_path,
        &running.cursor_path,
        &paths,
        running.screen,
        running.started_unix_ms,
    );

    let stopped = VideoSink::stop(running.video);
    let pick = |c: &AtomicU64| {
        let v = c.load(Ordering::SeqCst);
        (v > 0).then_some(v)
    };
    let segments = std::mem::take(&mut *running.segments.lock().unwrap_or_else(|e| e.into_inner()));
    save_session_files(
        &running.folder,
        stopped.frame_ts,
        running.events_ms,
        pick(&running.mic_start),
        pick(&running.system_start),
        running.screen,
        segments,
    );

    match stopped.error {
        Some(e) => Err(e),
        None => Ok(RecordingResult {
            folder: running.folder,
            frames: stopped.frames,
        }),
    }
}
