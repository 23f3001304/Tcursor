pub mod bitmap;
pub mod cursor;
pub mod hotkeys;
pub mod pointer;

use std::sync::Arc;

use crate::actions::matcher::Arm;
use crate::actions::model::ActionEvent;
use crate::events::model::MouseEvent;
use crate::events::remap::Remap;
use crate::events::track::cursortracker::CursorSamples;
use crate::ports::input::{CursorShapePort, HotkeyPort, InputPort, PointerPort};
use crate::session::record::pause_totals::PauseTotals;

pub use cursor::Win32CursorShapes;
pub use hotkeys::Win32Hotkeys;
pub use pointer::Win32Pointer;

pub struct Win32Input;

impl PointerPort for Win32Pointer {
    fn set_remap(&self, remap: Option<Remap>) {
        Win32Pointer::set_remap(self, remap);
    }

    fn stop(self: Box<Self>) -> Vec<MouseEvent> {
        Win32Pointer::stop(*self)
    }
}

impl HotkeyPort for Win32Hotkeys {
    fn stop(self: Box<Self>) -> (Vec<ActionEvent>, Vec<u32>) {
        Win32Hotkeys::stop(*self)
    }
}

impl CursorShapePort for Win32CursorShapes {
    fn stop(self: Box<Self>) -> CursorSamples {
        Win32CursorShapes::stop(*self)
    }
}

impl InputPort for Win32Input {
    fn pointer(&self, move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Box<dyn PointerPort> {
        Box::new(Win32Pointer::start(move_min_interval_ms, ledger))
    }

    fn hotkeys(&self, arms: Vec<Arm>, ledger: Arc<PauseTotals>) -> Box<dyn HotkeyPort> {
        Box::new(Win32Hotkeys::start(arms, ledger))
    }

    fn cursor_shapes(&self, ledger: Arc<PauseTotals>) -> Box<dyn CursorShapePort> {
        Box::new(Win32CursorShapes::start(ledger))
    }
}
