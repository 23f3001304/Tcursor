use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;

use crate::audio::cpal_mic::CpalMic;
use crate::audio::system_audio::SystemAudio;
use crate::domain::time::Clock;

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
            let handle = match SystemAudio::loopback(&system_path, paused) {
                Ok(h) => { started.store(clock.now_ms(), Ordering::SeqCst); Some(h) }
                Err(e) => { eprintln!("system-audio open failed (no loopback): {e}"); None }
            };
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if let Some(h) = handle { let _ = h.stop(); }
        })
        .ok()
}
