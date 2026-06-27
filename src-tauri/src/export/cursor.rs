use crate::events::model::{MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::FramePoint;

/// Cursor lookup that interpolates between throttled samples and then low-pass
/// smooths the result so small jittery mouse movements don't jerk the follow.
pub struct Cursor<'a> {
    events: &'a [MouseEvent],
    screen: &'a ScreenInfo,
    idx: usize,
    sx: f32,
    sy: f32,
    primed: bool,
}

impl<'a> Cursor<'a> {
    pub fn new(events: &'a [MouseEvent], screen: &'a ScreenInfo) -> Self {
        Self { events, screen, idx: 0, sx: 0.0, sy: 0.0, primed: false }
    }

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
            return to_frame(self.screen, x.round() as i32, y.round() as i32);
        }
        to_frame(self.screen, e0.x, e0.y)
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
        let mut c = Cursor::new(&ev, &s);
        assert_eq!(c.at(0), FramePoint { x: 960, y: 540 }); // before the first sample -> center
    }

    #[test]
    fn smooths_toward_a_jump_target() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let ev = vec![mv(0, 0, 0), mv(100, 800, 400)];
        let mut c = Cursor::new(&ev, &s);
        let mut p = FramePoint { x: 0, y: 0 };
        for t in (0..2000).step_by(16) { p = c.at(t); }
        assert!((p.x - 800).abs() <= 2 && (p.y - 400).abs() <= 2); // converged to the held target
    }
}
