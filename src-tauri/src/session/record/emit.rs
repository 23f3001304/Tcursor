use std::sync::Arc;
use tauri::Emitter;

use crate::audio::level::AudioLevel;
use crate::session::record::{Level, Notify};

pub struct TakeHooks {
    pub warn: Notify,
    pub ended: Notify,
    pub mic_level: Option<Level>,
    pub system_level: Option<Level>,
}

impl TakeHooks {
    pub fn from_app(app: &tauri::AppHandle) -> Self {
        Self {
            warn: emitter(app, "record-warning"),
            ended: emitter(app, "record-ended-early"),
            mic_level: Some(level_emitter(app, "mic")),
            system_level: Some(level_emitter(app, "system")),
        }
    }

    #[cfg(test)]
    pub fn silent() -> Self {
        Self {
            warn: Arc::new(|_| {}),
            ended: Arc::new(|_| {}),
            mic_level: None,
            system_level: None,
        }
    }
}

pub fn emitter(app: &tauri::AppHandle, event: &'static str) -> Notify {
    let app = app.clone();
    Arc::new(move |reason: &str| {
        let _ = app.emit(event, reason.to_string());
    })
}

pub fn level_emitter(app: &tauri::AppHandle, source: &'static str) -> Level {
    let app = app.clone();
    Arc::new(move |rms: f32| {
        let _ = app.emit("audio-level", AudioLevel { source, rms });
    })
}
