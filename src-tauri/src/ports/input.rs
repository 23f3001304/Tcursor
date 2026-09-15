use std::sync::Arc;

use crate::actions::matcher::Arm;
use crate::actions::model::ActionEvent;
use crate::events::model::MouseEvent;
use crate::events::remap::Remap;
use crate::events::track::cursortracker::CursorSamples;
use crate::session::record::pause_totals::PauseTotals;

pub trait PointerPort: Send {
    fn set_remap(&self, remap: Option<Remap>);
    fn stop(self: Box<Self>) -> Vec<MouseEvent>;
}

pub trait HotkeyPort: Send {
    fn stop(self: Box<Self>) -> (Vec<ActionEvent>, Vec<u32>);
}

pub trait CursorShapePort: Send {
    fn stop(self: Box<Self>) -> CursorSamples;
}

pub trait InputPort: Send + Sync {
    fn pointer(&self, move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Box<dyn PointerPort>;
    fn hotkeys(&self, arms: Vec<Arm>, ledger: Arc<PauseTotals>) -> Box<dyn HotkeyPort>;
    fn cursor_shapes(&self, ledger: Arc<PauseTotals>) -> Box<dyn CursorShapePort>;
}
