pub mod cursordraw;
pub mod cursorset;
pub mod cursorpreview;
pub mod pack;
pub mod pack_import;

use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::FramePoint;

/// Cursor lookup that interpolates between throttled samples and then low-pass
/// smooths the result so small jittery mouse movements don't jerk the follow.
/// Owns its event log so it can live inside the `FrameRenderer`. `at` must be
/// called with non-decreasing `t_ms` - the sample index only advances.
pub struct Cursor {
    events: Vec<MouseEvent>,
    screen: ScreenInfo,
    idx: usize,
    sx: f32,
    sy: f32,
    primed: bool,
}

impl Cursor {
    pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo) -> Self {
        Self { events, screen, idx: 0, sx: 0.0, sy: 0.0, primed: false }
    }

    /// The mouse events this cursor owns. Shared with FX rendering so the renderer
    /// has a single owner of the event log rather than a second copy.
    pub fn events(&self) -> &[MouseEvent] { &self.events }

    /// Click (mouse-down) positions as 0..1 fractions of the screen content, in the same
    /// coordinate basis as the smoothed cursor (`to_frame` then divide by the screen size),
    /// paired with each click's event time. The editor preview uses these for click ripples.
    pub fn clicks(&self) -> Vec<(u32, f32, f32)> {
        let (w, h) = (self.screen.w.max(1) as f32, self.screen.h.max(1) as f32);
        self.events.iter().filter(|e| e.kind == EventKind::Down).map(|e| {
            let p = to_frame(&self.screen, e.x, e.y);
            (e.t, (p.x as f32 / w).clamp(0.0, 1.0), (p.y as f32 / h).clamp(0.0, 1.0))
        }).collect()
    }

    /// Rewind the forward-only sample index + smoothing so the owning renderer can be
    /// reused to re-scan from t=0 (preview fast-forward to an arbitrary T).
    pub fn reset(&mut self) { self.idx = 0; self.sx = 0.0; self.sy = 0.0; self.primed = false; }

    /// Smoothed cursor position at `t_ms` (interpolate, then exponential low-pass).
    pub fn at(&mut self, t_ms: u32) -> FramePoint {
        while self.idx + 1 < self.events.len() && self.events[self.idx + 1].t <= t_ms {
            self.idx += 1;
        }
        let raw = self.raw_at(t_ms);
        const A: f32 = 0.35; // smoothing: lower = ignores more jitter
        if !self.primed {
            self.sx = raw.x as f32;
            self.sy = raw.y as f32;
            self.primed = true;
        } else {
            self.sx += (raw.x as f32 - self.sx) * A;
            self.sy += (raw.y as f32 - self.sy) * A;
        }
        FramePoint { x: self.sx.round() as i32, y: self.sy.round() as i32 }
    }

    /// Interpolated (un-smoothed) cursor; frame center before the first event.
    fn raw_at(&self, t_ms: u32) -> FramePoint {
        let e0 = match self.events.get(self.idx) {
            Some(e) if e.t <= t_ms => e,
            _ => return FramePoint { x: self.screen.w as i32 / 2, y: self.screen.h as i32 / 2 },
        };
        if let Some(e1) = self.events.get(self.idx + 1) {
            let span = (e1.t - e0.t).max(1) as f32;
            let f = ((t_ms - e0.t) as f32 / span).clamp(0.0, 1.0);
            let x = e0.x as f32 + (e1.x - e0.x) as f32 * f;
            let y = e0.y as f32 + (e1.y - e0.y) as f32 * f;
            return to_frame(&self.screen, x.round() as i32, y.round() as i32);
        }
        to_frame(&self.screen, e0.x, e0.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
    fn mv(t: u32, x: i32, y: i32) -> MouseEvent { MouseEvent { t, kind: EventKind::Move, x, y, button: None } }

    #[test]
    fn center_before_first_event() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let ev = vec![mv(1000, 100, 100)];
        let mut c = Cursor::new(ev, s);
        assert_eq!(c.at(0), FramePoint { x: 960, y: 540 }); // before the first sample -> center
    }

    #[test]
    fn smooths_toward_a_jump_target() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let ev = vec![mv(0, 0, 0), mv(100, 800, 400)];
        let mut c = Cursor::new(ev, s);
        let mut p = FramePoint { x: 0, y: 0 };
        for t in (0..2000).step_by(16) { p = c.at(t); }
        assert!((p.x - 800).abs() <= 2 && (p.y - 400).abs() <= 2); // converged to the held target
    }
}
