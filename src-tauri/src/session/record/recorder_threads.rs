use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;

use std::path::Path;

use crate::audio::capture::cpal_mic::CpalMic;
use crate::audio::capture::system_audio::SystemAudio;
use crate::domain::time::Clock;
use crate::actions::keyboard::KeyboardTracker;
use crate::actions::model::ActionLog;
use crate::events::track::cursortracker::CursorTypeTracker;
use crate::events::track::cursortype::CursorTrack;
use crate::events::model::{EventLog, ScreenInfo};
use crate::events::track::tracker::MouseTracker;
use crate::session::record::Notify;

/// The `record-warning` reason for an audio input that would not open (the selected mic held
/// exclusively by another app, or unplugged between the device list and pressing Record). The
/// take keeps running - this is not fatal - but it will be silent, and the user must not find
/// that out only when the editor opens on a twenty-minute walkthrough with no narration.
fn audio_warning(kind: &str, e: &impl std::fmt::Display) -> String {
    format!("No {kind} audio: that input could not be opened ({e}).")
}

/// Persist the recorded inputs (mouse events, keyboard actions/typing, cursor-type
/// timeline) to disk. Split out of `stop_recording` so that file stays under the cap.
/// Cursor track is written only for Enhanced recordings (tracker is `Some`).
pub fn save_inputs(
    mouse: Option<MouseTracker>,
    keyboard: Option<KeyboardTracker>,
    cursor: Option<CursorTypeTracker>,
    events_path: &Path,
    actions_path: &Path,
    typing_path: &Path,
    cursor_path: &Path,
    screen: ScreenInfo,
    started_unix_ms: u64,
) {
    if let Some(tracker) = mouse {
        let events = tracker.stop();
        let log = EventLog { started_unix_ms, screen, events };
        if let Err(e) = log.save(events_path) { eprintln!("events.json save failed: {e}"); }
    }
    if let Some(kb) = keyboard {
        let (actions, typing) = kb.stop();
        if let Err(e) = (ActionLog { actions }).save(actions_path) { eprintln!("actions.json save failed: {e}"); }
        crate::events::track::typing::TypingLog { ms: typing }.save(typing_path).ok();
    }
    if let Some(c) = cursor {
        let samples = c.stop();
        if let Err(e) = (CursorTrack { samples }).save(cursor_path) { eprintln!("cursor.json save failed: {e}"); }
    }
}

/// Persist the post-capture session files: `sync.json` (the real capture timeline the export
/// rebuilds its clocks from), the `.tcursor` project manifest, and the recents entry. Split out
/// of `stop_recording` so that file stays under the cap, alongside `save_inputs`. All three are
/// best-effort - a failure is logged, never fails the stop; the folder is still a fully valid
/// project without the manifest, just not open-project-able by dialog until the NEXT write.
///
/// `preprocessed: false` here on purpose: the frontend calls `preprocess_project` right after
/// `stop_recording` resolves (shown as the HUD's "Saving..." progress) and that flips it once the
/// pass finishes. NOT done here as a detached background thread anymore - that used to race the
/// editor's mount (it could still be transcoding when the editor opened), which is exactly the
/// "preview still takes a while to load" lag; awaiting it with progress in the caller fixes that.
pub fn save_session_files(folder: &str, frames: Vec<u64>, events_ms: u64,
    mic_ms: Option<u64>, system_ms: Option<u64>, screen: ScreenInfo) {
    let sync = crate::session::sync::SyncLog { frames, events_ms, mic_ms, system_ms };
    if let Err(e) = sync.save(&Path::new(folder).join("sync.json")) { eprintln!("sync.json save failed: {e}"); }
    let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(folder) };
    let manifest = crate::session::project::manifest::ProjectManifest::new(screen.w, screen.h);
    if let Err(e) = manifest.save(&paths.manifest()) { eprintln!("project.tcursor save failed: {e}"); }
    crate::session::project::recents::touch(folder); // also makes fresh recordings show up as "recent"
}

/// Spawn the mic recording thread. `mic_id: None` means mic off — returns `None` immediately.
/// cpal::Stream is !Send so mic must be created and destroyed on its own thread.
pub fn spawn_mic_thread(
    mic_id: Option<String>,
    mic_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
    warn: Notify,
) -> Option<JoinHandle<()>> {
    let id = mic_id?;
    std::thread::Builder::new()
        .name("mic".into())
        .spawn(move || {
            // mic_start is stamped inside the callback at the first sample's CAPTURE
            // time (see CpalMic::open), cancelling the device input latency.
            let handle = match CpalMic::open(Some(&id), &mic_path, paused, started, clock) {
                Ok(h) => Some(h),
                // The WAV is created before the input stream is built, so a failure here can
                // leave a header-only mic.wav that `build_timeline` would treat as real audio
                // and mux at a bogus offset. Remove it so the project is honestly silent.
                Err(e) => { let _ = std::fs::remove_file(&mic_path); warn(&audio_warning("microphone", &e)); None }
            };

            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if let Some(h) = handle { let _ = h.stop(); }
        })
        .ok()
}

/// Spawn the system-audio loopback thread. Returns `None` if `enabled` is false.
/// SystemAudioHandle contains cpal::Stream (!Send) so must live on its own thread.
pub fn spawn_system_thread(
    enabled: bool,
    system_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
    warn: Notify,
) -> Option<JoinHandle<()>> {
    if !enabled { return None; }
    std::thread::Builder::new()
        .name("system-audio".into())
        .spawn(move || {
            // system_start is stamped inside the callback at the first non-empty packet
            // (see SystemAudio::loopback), mirroring the mic's in-callback pattern instead
            // of stamping here at stream-open (which lands earlier than samples actually
            // start arriving).
            let handle = match SystemAudio::loopback(&system_path, paused, started, clock) {
                Ok(h) => Some(h),
                // Same header-only-WAV cleanup as the mic thread above.
                Err(e) => { let _ = std::fs::remove_file(&system_path); warn(&audio_warning("system", &e)); None }
            };
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if let Some(h) = handle { let _ = h.stop(); }
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
