pub mod recorder;
pub mod recorder_stop;
pub mod close_guard;
pub mod recorder_threads;
pub mod recording_session;
pub mod dim_guard;
pub mod gpu_frames;
pub mod gpu_record;
pub mod video_sink;
mod pause_clock;
pub mod pause_totals;

use std::sync::Arc;

/// A one-way notification from a recording thread to the app: the reason string is what the
/// HUD shows the user. An `Arc<dyn Fn>` rather than an `AppHandle` so the capture/encode
/// pipeline keeps no Tauri types and stays constructible in tests.
pub type Notify = Arc<dyn Fn(&str) + Send + Sync>;

/// Reason passed to the capture-ended `Notify` when the OS - not the user - ends the capture:
/// the recorded window was closed, or the recorded display was unplugged/disabled/slept. Both
/// capture paths report it with the same wording so the HUD has one message to show.
pub const CAPTURE_CLOSED: &str = "The recorded window or display closed. The recording was saved up to that point.";

/// Reason passed to the same `Notify` when the capture's OWN dimensions change mid-record - a
/// recorded window maximized/restored/snapped, or a recorded display changed resolution,
/// rotated, or was docked/undocked. Neither capture path can keep encoding once that happens
/// (the encoder/pipe is sized once, at start - finding H1): rather than the legacy path
/// silently discarding every frame from that instant on, or the GPU path never even checking,
/// both now end the take through this same early-end signal on the FIRST mismatched frame, and
/// this distinct wording is what tells the HUD (and the user) it was a size change, not a
/// closed window or display.
pub const DISPLAY_CHANGED: &str = "Display changed — recording saved up to the change.";
