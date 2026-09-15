pub mod cursorpreview;
pub mod draw;
pub mod pack;
pub mod path;

// SEAM: everything outside this folder names these modules at `export::cursor::<name>`.
pub use draw::{busy, captured, cursordraw, cursormorph};
pub use pack::{cursorset, pack_import, pack_template};

use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::cursor::draw::tilt;
use crate::export::types::FramePoint;

pub struct Cursor {
    events: Vec<MouseEvent>,
    screen: ScreenInfo,
    idx: usize,
    path: path::PathModel,
    smooth: f32,
    ideal: f32,
    tilt: tilt::Tilt,
    tilt_max: f32,
}

impl Cursor {
    pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo, smoothness: f32) -> Self {
        let path = path::PathModel::new(&events, &screen);
        Self {
            events,
            screen,
            idx: 0,
            path,
            smooth: smoothness.clamp(0.0, 1.0),
            ideal: 0.0,
            tilt: tilt::Tilt::new(),
            tilt_max: 0.0,
        }
    }

    pub fn set_smoothness(&mut self, s: f32) {
        self.smooth = s.clamp(0.0, 1.0);
    }

    pub fn set_idealize(&mut self, s: f32) {
        self.ideal = s.clamp(0.0, 1.0);
    }

    pub fn set_tilt(&mut self, t: f32) {
        self.tilt_max = tilt::max_deg(t);
        if self.tilt_max <= 0.0 {
            self.tilt.reset();
        }
    }

    pub fn tilt_deg(&self) -> f32 {
        self.tilt.angle_deg()
    }

    pub fn events(&self) -> &[MouseEvent] {
        &self.events
    }

    pub fn screen(&self) -> ScreenInfo {
        self.screen
    }

    pub fn clicks(&self) -> Vec<(u32, f32, f32)> {
        let (w, h) = (self.screen.w.max(1) as f32, self.screen.h.max(1) as f32);
        self.events
            .iter()
            .filter(|e| e.kind == EventKind::Down)
            .map(|e| {
                let p = to_frame(&self.screen, e.x, e.y);
                (
                    e.t,
                    (p.x as f32 / w).clamp(0.0, 1.0),
                    (p.y as f32 / h).clamp(0.0, 1.0),
                )
            })
            .collect()
    }

    pub fn reset(&mut self) {
        self.idx = 0;
        self.tilt.reset();
    }

    pub fn at(&mut self, t_ms: u32, dt_ms: f32) -> FramePoint {
        while self.idx + 1 < self.events.len() && self.events[self.idx + 1].t <= t_ms {
            self.idx += 1;
        }
        let raw = self.raw_at(t_ms);
        let before_first = self.events.first().map_or(true, |e| e.t > t_ms);
        let (fx, fy) = if before_first || (self.smooth <= 0.0 && self.ideal <= 0.0) {
            (raw.x as f32, raw.y as f32)
        } else {
            self.path
                .at(t_ms, self.smooth, self.ideal)
                .unwrap_or((raw.x as f32, raw.y as f32))
        };
        if self.tilt_max > 0.0 {
            let k = tilt::ref_scale(self.screen.w);
            self.tilt.step(fx * k, fy * k, dt_ms, self.tilt_max);
        }
        FramePoint {
            x: fx.round() as i32,
            y: fy.round() as i32,
        }
    }

    fn raw_at(&self, t_ms: u32) -> FramePoint {
        let e0 = match self.events.get(self.idx) {
            Some(e) if e.t <= t_ms => e,
            _ => {
                return FramePoint {
                    x: self.screen.w as i32 / 2,
                    y: self.screen.h as i32 / 2,
                }
            }
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
#[path = "mod_tests.rs"]
mod tests;
