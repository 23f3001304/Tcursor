use std::sync::Arc;
use tauri::Emitter;

use crate::audio::level::AudioLevel;
use crate::session::record::{Level, Notify};

/// A `Notify` that forwards its reason to the frontend as `event`. Recording threads hold these
/// instead of an `AppHandle`, so nothing below this file needs to know about Tauri.
pub fn emitter(app: &tauri::AppHandle, event: &'static str) -> Notify {
    let app = app.clone();
    Arc::new(move |reason: &str| { let _ = app.emit(event, reason.to_string()); })
}

/// A `Level` that forwards each reading to the frontend as `audio-level`, tagged with which
/// capture it came from. Emitted from the capture thread's own poll loop (never from the audio
/// callback), so the realtime thread stays free of IPC.
pub fn level_emitter(app: &tauri::AppHandle, source: &'static str) -> Level {
    let app = app.clone();
    Arc::new(move |rms: f32| { let _ = app.emit("audio-level", AudioLevel { source, rms }); })
}
