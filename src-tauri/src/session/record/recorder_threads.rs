use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use std::path::Path;

use crate::actions::model::ActionLog;
use crate::audio::capture::cpal_mic::CpalMic;
use crate::audio::capture::system_audio::SystemAudio;
use crate::audio::level::LevelSlot;
use crate::domain::time::Clock;
use crate::events::model::{EventLog, ScreenInfo};
use crate::events::track::cursortype::CursorTrack;
use crate::ports::input::{CursorShapePort, HotkeyPort, PointerPort};
use crate::session::record::Level;
use crate::session::record::Notify;

fn audio_warning(kind: &str, e: &impl std::fmt::Display) -> String {
    format!("No {kind} audio: that input could not be opened ({e}).")
}

pub const LEVEL_POLL_MS: u64 = 50;

fn poll_until_stopped(stop: &AtomicBool, slot: &Arc<LevelSlot>, level: &Option<Level>) {
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(LEVEL_POLL_MS));
        if let Some(l) = level {
            l(slot.take());
        }
    }
}

pub fn save_inputs(
    mouse: Box<dyn PointerPort>,
    keyboard: Box<dyn HotkeyPort>,
    cursor: Box<dyn CursorShapePort>,
    events_path: &Path,
    actions_path: &Path,
    typing_path: &Path,
    cursor_path: &Path,
    paths: &crate::session::paths::ProjectPaths,
    screen: ScreenInfo,
    started_unix_ms: u64,
) {
    let events = PointerPort::stop(mouse);
    let log = EventLog {
        started_unix_ms,
        screen,
        events,
    };
    if let Err(e) = log.save(events_path) {
        eprintln!("events.json save failed: {e}");
    }
    let (actions, typing) = HotkeyPort::stop(keyboard);
    if let Err(e) = (ActionLog { actions }).save(actions_path) {
        eprintln!("actions.json save failed: {e}");
    }
    crate::events::track::typing::TypingLog { ms: typing }
        .save(typing_path)
        .ok();
    let (samples, layer) = CursorShapePort::stop(cursor);
    if let Err(e) = (CursorTrack { samples }).save(cursor_path) {
        eprintln!("cursor.json save failed: {e}");
    }
    if let Err(e) = layer.save(paths) {
        eprintln!("cursor layer save failed: {e}");
    }
}

pub fn save_session_files(
    folder: &str,
    frames: Vec<u64>,
    events_ms: u64,
    mic_ms: Option<u64>,
    system_ms: Option<u64>,
    screen: ScreenInfo,
    segments: super::segments::SegmentLog,
) {
    let sync = crate::session::sync::SyncLog {
        frames,
        events_ms,
        mic_ms,
        system_ms,
        mic_segments: segments.mic,
        webcam_segments: segments.webcam,
        display_switches: segments.displays,
    };
    if let Err(e) = sync.save(&Path::new(folder).join("sync.json")) {
        eprintln!("sync.json save failed: {e}");
    }
    let paths = crate::session::paths::ProjectPaths {
        folder: std::path::PathBuf::from(folder),
    };
    let manifest = crate::session::project::manifest::ProjectManifest::new(screen.w, screen.h);
    if let Err(e) = manifest.save(&paths.manifest()) {
        eprintln!("project.tcursor save failed: {e}");
    }
    crate::session::project::recents::touch(folder);
}

pub fn spawn_mic_thread(
    mic_id: Option<String>,
    mic_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
    warn: Notify,
    level: Option<Level>,
) -> Option<JoinHandle<()>> {
    let id = mic_id?;
    std::thread::Builder::new()
        .name("mic".into())
        .spawn(move || {
            let slot = Arc::new(LevelSlot::new());
            let handle = match CpalMic::open(
                Some(&id),
                &mic_path,
                paused,
                started,
                clock,
                Some(slot.clone()),
            ) {
                Ok(h) => Some(h),
                Err(e) => {
                    let _ = std::fs::remove_file(&mic_path);
                    warn(&audio_warning("microphone", &e));
                    None
                }
            };
            poll_until_stopped(&stop, &slot, &if handle.is_some() { level } else { None });
            if let Some(h) = handle {
                let _ = h.stop();
            }
        })
        .ok()
}

pub fn spawn_system_thread(
    loopback: Option<(cpal::Device, cpal::SupportedStreamConfig)>,
    system_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
    warn: Notify,
    level: Option<Level>,
) -> Option<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("system-audio".into())
        .spawn(move || {
            let slot = Arc::new(LevelSlot::new());
            let handle = match SystemAudio::loopback(
                loopback,
                &system_path,
                paused,
                started,
                clock,
                Some(slot.clone()),
            ) {
                Ok(h) => Some(h),
                Err(e) => {
                    let _ = std::fs::remove_file(&system_path);
                    warn(&audio_warning("system", &e));
                    None
                }
            };
            poll_until_stopped(&stop, &slot, &if handle.is_some() { level } else { None });
            if let Some(h) = handle {
                let _ = h.stop();
            }
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_warning_names_the_input_and_carries_the_cause() {
        let msg = audio_warning("microphone", &"device in use by another app");
        assert!(msg.contains("microphone"), "{msg}");
        assert!(msg.contains("device in use by another app"), "{msg}");
    }
}
