use std::sync::{Arc, Mutex};

use crate::events::track::cursorlayer::CursorLayerBuilder;
use crate::events::track::cursortracker::CursorSamples;
use crate::platform::Platform;
use crate::ports::audio::SystemAudioPort;
use crate::ports::capture::{
    CaptureGeometry, CapturePort, CaptureRequest, CaptureTarget, FirstFrameSize, TargetId,
    TargetKind, VideoSink,
};
use crate::ports::input::{CursorShapePort, HotkeyPort, InputPort, PointerPort};
use crate::ports::system::{SystemPort, WindowHandle};
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::video_sink::VideoStopped;

pub type Calls = Arc<Mutex<Vec<String>>>;

pub const GEOMETRY: CaptureGeometry = CaptureGeometry {
    w: 1920,
    h: 1080,
    origin_x: 0,
    origin_y: 0,
};

pub fn platform() -> (Platform, Calls) {
    let calls: Calls = Arc::new(Mutex::new(Vec::new()));
    let p = Platform {
        capture: Box::new(MockCapture(calls.clone())),
        input: Box::new(MockInput(calls.clone())),
        system: Box::new(MockSystem),
        audio: Box::new(MockAudio),
    };
    (p, calls)
}

fn note(calls: &Calls, what: &str) {
    calls
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(what.to_string());
}

pub struct MockCapture(Calls);

pub struct MockSink(Calls);

impl VideoSink for MockSink {
    fn switch(
        &mut self,
        req: CaptureRequest,
        on_first_frame: FirstFrameSize,
    ) -> Result<(), String> {
        note(&self.0, &format!("switch:{}", req.target));
        if let Some(hook) = on_first_frame {
            hook(GEOMETRY.w, GEOMETRY.h);
        }
        Ok(())
    }

    fn stop(self: Box<Self>) -> VideoStopped {
        note(&self.0, "stop");
        VideoStopped {
            frames: 2,
            frame_ts: vec![0, 16],
            error: None,
        }
    }

    fn supports_switch(&self) -> bool {
        true
    }
}

impl CapturePort for MockCapture {
    fn list_targets(&self) -> Vec<CaptureTarget> {
        vec![CaptureTarget {
            id: TargetId::Primary,
            label: "Mock Display".into(),
            kind: TargetKind::Display,
        }]
    }

    fn bounds(&self, _target: &TargetId) -> CaptureGeometry {
        GEOMETRY
    }

    fn start(&self, req: CaptureRequest) -> Result<(Box<dyn VideoSink>, CaptureGeometry), String> {
        note(&self.0, &format!("start:{}", req.target));
        Ok((Box::new(MockSink(self.0.clone())), GEOMETRY))
    }
}

pub struct MockInput(Calls);

pub struct MockPointer;

pub struct MockHotkeys;

pub struct MockCursorShapes;

impl PointerPort for MockPointer {
    fn set_remap(&self, _remap: Option<crate::events::remap::Remap>) {}

    fn stop(self: Box<Self>) -> Vec<crate::events::model::MouseEvent> {
        Vec::new()
    }
}

impl HotkeyPort for MockHotkeys {
    fn stop(self: Box<Self>) -> (Vec<crate::actions::model::ActionEvent>, Vec<u32>) {
        (Vec::new(), Vec::new())
    }
}

impl CursorShapePort for MockCursorShapes {
    fn stop(self: Box<Self>) -> CursorSamples {
        (Vec::new(), CursorLayerBuilder::default())
    }
}

impl InputPort for MockInput {
    fn pointer(&self, _min_ms: u32, _ledger: Arc<PauseTotals>) -> Box<dyn PointerPort> {
        note(&self.0, "pointer");
        Box::new(MockPointer)
    }

    fn hotkeys(
        &self,
        _arms: Vec<crate::actions::matcher::Arm>,
        _ledger: Arc<PauseTotals>,
    ) -> Box<dyn HotkeyPort> {
        note(&self.0, "hotkeys");
        Box::new(MockHotkeys)
    }

    fn cursor_shapes(&self, _ledger: Arc<PauseTotals>) -> Box<dyn CursorShapePort> {
        note(&self.0, "cursor_shapes");
        Box::new(MockCursorShapes)
    }
}

pub struct MockSystem;

impl SystemPort for MockSystem {
    fn primary_refresh_hz(&self) -> u32 {
        60
    }

    fn os_prefers_dark(&self) -> bool {
        false
    }

    fn exclude_from_capture(&self, _window: WindowHandle, _exclude: bool) -> bool {
        true
    }
}

pub struct MockAudio;

impl SystemAudioPort for MockAudio {
    fn loopback_device(&self) -> Option<(cpal::Device, cpal::SupportedStreamConfig)> {
        None
    }
}

#[cfg(test)]
#[path = "cycle_tests.rs"]
mod tests;
