pub mod busy;
pub mod captured;
pub mod cursordraw;
pub mod cursormorph;
pub mod cursorset;
pub mod cursorxform;
pub mod cursorpreview;
pub mod pack;
pub mod packdirs;
pub mod packlist;
pub mod pack_import;
pub mod path;
pub mod tilt;

use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::FramePoint;

/// Cursor lookup: the raw recorded path, or the polished one `path::PathModel` draws between the
/// points the recording actually rested and clicked at. Owns its event log so it can live inside
/// the `FrameRenderer`. `at` must be called with non-decreasing `t_ms` - the raw sample index only
/// advances (`reset` rewinds it).
pub struct Cursor {
    events: Vec<MouseEvent>,
    screen: ScreenInfo,
    idx: usize,
    path: path::PathModel, // the offline rest/move model the polished path is drawn from
    smooth: f32,           // `CursorSettings::smoothness`, 0..1 (0 = the raw recording's timing)
    ideal: f32,            // `CursorSettings::path_idealize`, 0..1 (0 = the raw route)
    tilt: tilt::Tilt,      // the motion-lean filter, fed the path this struct returns
    tilt_max: f32,         // its cap in degrees (`tilt::max_deg` of the setting); 0 = filter off
}

impl Cursor {
    pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo, smoothness: f32) -> Self {
        let path = path::PathModel::new(&events, &screen);
        Self { events, screen, idx: 0, path, smooth: smoothness.clamp(0.0, 1.0), ideal: 0.0,
               tilt: tilt::Tilt::new(), tilt_max: 0.0 }
    }

    /// Update the glide strength in place so a `smoothness` settings change reflects via
    /// `FrameRenderer::reload_edit` without rebuilding the whole cursor.
    pub fn set_smoothness(&mut self, s: f32) { self.smooth = s.clamp(0.0, 1.0); }

    /// Update the path-idealization strength (0 = the raw route, 1 = straight eased strokes between
    /// the rests). Settings-driven, live-applied like `set_smoothness`.
    pub fn set_idealize(&mut self, s: f32) { self.ideal = s.clamp(0.0, 1.0); }

    /// Update the motion-tilt strength (`CursorSettings::tilt`, 0..1). Settings-driven and
    /// live-applied like `set_smoothness`. Turning it off also unwinds the filter, so the cursor
    /// cannot be left frozen mid-lean by a slider drag through 0.
    pub fn set_tilt(&mut self, t: f32) {
        self.tilt_max = tilt::max_deg(t);
        if self.tilt_max <= 0.0 { self.tilt.reset(); }
    }

    /// This frame's motion lean in degrees, clockwise-positive - what `cursorset::draw` rotates the
    /// sprite by. Valid after `at`, which is what advances it.
    pub fn tilt_deg(&self) -> f32 { self.tilt.angle_deg() }

    /// The mouse events this cursor owns. Shared with FX rendering so the renderer
    /// has a single owner of the event log rather than a second copy.
    pub fn events(&self) -> &[MouseEvent] { &self.events }

    /// The recording's `ScreenInfo` (virtual-desktop origin). `Copy`, so this is a cheap read, not
    /// a second owner - shared with FX rendering (`fx_state::render`) so a click hit's raw
    /// `WH_MOUSE_LL` coordinates go through the same origin subtraction (`coordmap::to_frame`)
    /// this cursor already applies to itself (see `clicks`, `path::PathModel::new`).
    pub fn screen(&self) -> ScreenInfo { self.screen }

    /// Click (mouse-down) positions as 0..1 fractions of the screen content, in the same
    /// coordinate basis as the drawn cursor (`to_frame` then divide by the screen size),
    /// paired with each click's event time. The editor preview uses these for click ripples.
    pub fn clicks(&self) -> Vec<(u32, f32, f32)> {
        let (w, h) = (self.screen.w.max(1) as f32, self.screen.h.max(1) as f32);
        self.events.iter().filter(|e| e.kind == EventKind::Down).map(|e| {
            let p = to_frame(&self.screen, e.x, e.y);
            (e.t, (p.x as f32 / w).clamp(0.0, 1.0), (p.y as f32 / h).clamp(0.0, 1.0))
        }).collect()
    }

    /// Rewind the forward-only sample index + the lean so the owning renderer can be reused to
    /// re-scan from t=0 (preview fast-forward to an arbitrary T). The path model is stateless.
    pub fn reset(&mut self) {
        self.idx = 0;
        self.tilt.reset(); // no arriving at the far side of a cut still leaning from the gesture before it
    }

    /// The drawn cursor position at `t_ms` (frame-local px). With no polish asked for (plain-OS
    /// mode, or both sliders at 0) this is the raw interpolated recording, verbatim; otherwise the
    /// path model's polished route, which still passes through every rest and click exactly.
    /// `dt_ms` is the caller's EXACT frame period (`render::OUT_STEP_MS`), which only the motion
    /// lean consumes: the path itself is a function of time, so it never lags and never depends on
    /// the output rate.
    pub fn at(&mut self, t_ms: u32, dt_ms: f32) -> FramePoint {
        while self.idx + 1 < self.events.len() && self.events[self.idx + 1].t <= t_ms {
            self.idx += 1;
        }
        let raw = self.raw_at(t_ms);
        let before_first = self.events.first().map_or(true, |e| e.t > t_ms);
        let (fx, fy) = if before_first || (self.smooth <= 0.0 && self.ideal <= 0.0) {
            (raw.x as f32, raw.y as f32)
        } else {
            self.path.at(t_ms, self.smooth, self.ideal).unwrap_or((raw.x as f32, raw.y as f32))
        };
        // The lean reads the path the cursor is actually DRAWN on, so an idealized stroke leans into
        // its own clean route, not the wander the viewer never sees. Skipped when the setting is 0.
        if self.tilt_max > 0.0 {
            let k = tilt::ref_scale(self.screen.w);
            self.tilt.step(fx * k, fy * k, dt_ms, self.tilt_max);
        }
        FramePoint { x: fx.round() as i32, y: fy.round() as i32 }
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
#[path = "mod_tests.rs"]
mod tests;
