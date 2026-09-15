use std::convert::Infallible;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::domain::time::Clock;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::video_sink::VideoStopped;
use crate::session::record::Notify;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetId {
    Primary,
    Display(usize),
    Window(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Display,
    Window,
}

pub struct CaptureTarget {
    pub id: TargetId,
    pub label: String,
    pub kind: TargetKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureGeometry {
    pub w: u32,
    pub h: u32,
    pub origin_x: i32,
    pub origin_y: i32,
}

pub type FirstFrameSize = Option<Box<dyn FnOnce(u32, u32) + Send>>;

pub struct CaptureRequest {
    pub target: TargetId,
    pub output: PathBuf,
    pub fps: u32,
    pub with_cursor: bool,
    pub prefer_compatibility: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
}

pub trait VideoSink: Send {
    fn switch(&mut self, req: CaptureRequest, on_first_frame: FirstFrameSize)
        -> Result<(), String>;
    fn stop(self: Box<Self>) -> VideoStopped;
    fn supports_switch(&self) -> bool;
}

pub trait CapturePort: Send + Sync {
    fn list_targets(&self) -> Vec<CaptureTarget>;
    fn bounds(&self, target: &TargetId) -> CaptureGeometry;
    fn start(&self, req: CaptureRequest) -> Result<(Box<dyn VideoSink>, CaptureGeometry), String>;
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetId::Primary => f.write_str("primary"),
            TargetId::Display(i) => write!(f, "display:{i}"),
            TargetId::Window(h) => write!(f, "window:0x{h:x}"),
        }
    }
}

impl FromStr for TargetId {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(hex) = s.strip_prefix("window:0x") {
            if let Ok(h) = u64::from_str_radix(hex, 16) {
                return Ok(TargetId::Window(h));
            }
        } else if let Some(idx) = s.strip_prefix("display:") {
            if let Ok(i) = idx.parse::<usize>() {
                return Ok(TargetId::Display(i));
            }
        }
        Ok(TargetId::Primary)
    }
}

impl TargetId {
    pub fn as_arg(&self) -> Option<String> {
        match self {
            TargetId::Primary => None,
            other => Some(other.to_string()),
        }
    }

    pub fn from_arg(arg: Option<&str>) -> Self {
        arg.map_or(TargetId::Primary, |s| {
            s.parse().unwrap_or(TargetId::Primary)
        })
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
