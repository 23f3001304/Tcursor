pub mod close_guard;
pub mod emit;
pub mod frame_fit;
pub mod pause_clock;
pub mod pause_totals;
pub mod recorder;
pub mod recorder_stop;
pub mod recorder_threads;
pub mod recording_session;
pub mod segments;
pub mod switch_display;
pub mod switch_mic;
pub mod video_sink;
pub mod webcam_segments;

use std::sync::Arc;

pub type Notify = Arc<dyn Fn(&str) + Send + Sync>;

pub type Level = Arc<dyn Fn(f32) + Send + Sync>;

pub const CAPTURE_CLOSED: &str =
    "The recorded window or display closed. The recording was saved up to that point.";

pub const DISPLAY_CHANGED: &str = "Display changed. Recording saved up to the change.";
