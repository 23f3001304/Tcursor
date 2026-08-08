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

/// Spawn the mic recording thread. `mic_id: None` means mic off — returns `None` immediately.
/// cpal::Stream is !Send so mic must be created and destroyed on its own thread.
pub fn spawn_mic_thread(
    mic_id: Option<String>,
    mic_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
) -> Option<JoinHandle<()>> {
    let id = mic_id?;
    std::thread::Builder::new()
        .name("mic".into())
        .spawn(move || {
            // mic_start is stamped inside the callback at the first sample's CAPTURE
            // time (see CpalMic::open), cancelling the device input latency.
            let handle = match CpalMic::open(Some(&id), &mic_path, paused, started, clock) {
                Ok(h) => Some(h),
                Err(e) => { eprintln!("mic open failed (no audio): {e}"); None }
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
                Err(e) => { eprintln!("system-audio open failed (no loopback): {e}"); None }
            };
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if let Some(h) = handle { let _ = h.stop(); }
        })
        .ok()
}
