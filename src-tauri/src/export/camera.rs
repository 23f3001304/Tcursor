use crate::export::types::{Camera, Easing, FramePoint, ZoomConfig, ZoomRegion};

/// Normalized easing curve `[0,1] -> [0,1]`. `Smooth` = smoothstep; `Linear` = identity;
/// `Spring` = ease-out-back (a small overshoot past 1 near the end, then settle).
fn ease(e: Easing, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    match e {
        Easing::Linear => p,
        Easing::Smooth => p * p * (3.0 - 2.0 * p),
        Easing::Spring { .. } => { let c = 1.70158; let q = p - 1.0; 1.0 + (c + 1.0) * q * q * q + c * q * q }
    }
}

/// Shrink `(zi, zo)` proportionally so `zi + zo <= span`, keeping both ramps inside the pill.
fn fit_durations(zi: u32, zo: u32, span: u32) -> (u32, u32) {
    let total = zi + zo;
    if total <= span || total == 0 { return (zi, zo); }
    let f = span as f32 / total as f32;
    ((zi as f32 * f) as u32, (zo as f32 * f) as u32)
}

/// A virtual camera whose zoom scale follows a deterministic eased curve contained
/// entirely within each zoom region: it ramps 1 -> target over `zoom_in_ms`, holds,
/// then ramps target -> 1 over `zoom_out_ms`, reaching 1 exactly at `end_ms` (so the
/// timeline pill is an honest bound). The center eases toward the click point in
/// lockstep on zoom-in, pans to keep the cursor in view during hold, and is recentred
/// by the in-frame clamp as scale returns to 1. Most-recent active region wins.
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32 }

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self { frame_w, frame_h, cx: frame_w as f32 / 2.0, cy: frame_h as f32 / 2.0, scale: 1.0 }
    }

    pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera {
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        // Most-recent active region wins so a fresh click takes over immediately.
        let active = regions.iter().rev().find(|r| t_ms >= r.start_ms && t_ms <= r.end_ms);
        match active {
            None => { self.scale = 1.0; self.cx = fw / 2.0; self.cy = fh / 2.0; }
            Some(r) => {
                let span = r.end_ms.saturating_sub(r.start_ms).max(1);
                let (zi, zo) = fit_durations(r.zoom_in_ms, r.zoom_out_ms, span);
                let (zin_end, zout_start) = (r.start_ms + zi, r.end_ms.saturating_sub(zo));
                let s = r.target_scale;
                if t_ms < zin_end {
                    // zoom in: scale + center ease together toward the click point (lockstep).
                    let e = ease(r.easing, (t_ms - r.start_ms) as f32 / zi.max(1) as f32);
                    self.scale = 1.0 + (s - 1.0) * e;
                    self.cx = fw / 2.0 + (r.anchor.x as f32 - fw / 2.0) * e;
                    self.cy = fh / 2.0 + (r.anchor.y as f32 - fh / 2.0) * e;
                } else if t_ms >= zout_start {
                    // zoom out: contained in the pill; scale hits 1.0 exactly at end_ms. Center held
                    // (the in-frame clamp below recenters as scale -> 1, so its value is moot).
                    let e = ease(r.easing, (r.end_ms - t_ms) as f32 / zo.max(1) as f32);
                    self.scale = 1.0 + (s - 1.0) * e;
                } else {
                    // hold: fixed target scale; pan only when the cursor nears a viewport edge.
                    self.scale = s;
                    let (mx, my) = (fw / (2.0 * s) * 0.4, fh / (2.0 * s) * 0.4);
                    let (dx, dy) = (cursor.x as f32 - self.cx, cursor.y as f32 - self.cy);
                    let tx = if dx > mx { cursor.x as f32 - mx } else if dx < -mx { cursor.x as f32 + mx } else { self.cx };
                    let ty = if dy > my { cursor.y as f32 - my } else if dy < -my { cursor.y as f32 + my } else { self.cy };
                    let k = cfg.follow_damping.clamp(0.0, 1.0);
                    self.cx += (tx - self.cx) * k;
                    self.cy += (ty - self.cy) * k;
                }
            }
        }
        // Clamp the center so the view (frame / current scale) stays in-frame.
        let (half_w, half_h) = (fw / (2.0 * self.scale.max(0.01)), fh / (2.0 * self.scale.max(0.01)));
        self.cx = self.cx.clamp(half_w, (fw - half_w).max(half_w));
        self.cy = self.cy.clamp(half_h, (fh - half_h).max(half_h));
        Camera { cx: self.cx, cy: self.cy, scale: self.scale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

    fn region() -> ZoomRegion {
        ZoomRegion { start_ms: 0, end_ms: 2000, zoom_in_ms: 300, zoom_out_ms: 300,
            target_scale: 2.0, anchor: FramePoint { x: 400, y: 300 }, easing: Easing::Smooth }
    }

    #[test]
    fn no_region_is_full_frame() {
        let mut s = CameraSim::new(800, 600);
        let c = s.step(0, FramePoint { x: 400, y: 300 }, &[], &ZoomConfig::default());
        assert_eq!(c.scale, 1.0);
        assert!((c.cx - 400.0).abs() < 1.0 && (c.cy - 300.0).abs() < 1.0);
    }

    #[test]
    fn scale_is_one_at_region_end_and_after() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![region()];
        // walk up to the end so the (stateful) center advances naturally
        let mut sc = 0.0;
        for t in (0..=2000).step_by(16) { sc = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg).scale; }
        assert!((s.step(2000, FramePoint { x: 400, y: 300 }, &r, &cfg).scale - 1.0).abs() < 1e-3, "scale must be exactly 1 at end_ms");
        assert!((s.step(2100, FramePoint { x: 400, y: 300 }, &[], &cfg).scale - 1.0).abs() < 1e-3, "scale stays 1 after the region");
        let _ = sc;
    }

    #[test]
    fn scale_reaches_target_during_hold() {
        let mut s = CameraSim::new(800, 600);
        let c = s.step(1000, FramePoint { x: 400, y: 300 }, &[region()], &ZoomConfig::default());
        assert!((c.scale - 2.0).abs() < 1e-3, "hold scale equals target");
    }

    #[test]
    fn ramp_in_is_monotonic_and_bounded() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![region()];
        let a = s.step(0, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
        let b = s.step(150, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
        assert!(a < b && b < 2.0 && a >= 1.0, "in-ramp climbs from 1 toward target: a={a} b={b}");
    }

    #[test]
    fn durations_that_exceed_span_are_scaled_to_fit() {
        // in+out = 600 > span 400: must still reach 1.0 exactly at end and never NaN.
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![ZoomRegion { start_ms: 0, end_ms: 400, ..region() }];
        for t in (0..=400).step_by(16) { let _ = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg); }
        assert!((s.step(400, FramePoint { x: 400, y: 300 }, &r, &cfg).scale - 1.0).abs() < 1e-3);
    }

    #[test]
    fn newer_region_preempts_older_overlap() {
        // Two overlapping regions: at t in both, the later-starting one wins.
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let a = ZoomRegion { start_ms: 0, end_ms: 2000, anchor: FramePoint { x: 100, y: 100 }, ..region() };
        let b = ZoomRegion { start_ms: 1000, end_ms: 3000, anchor: FramePoint { x: 700, y: 500 }, ..region() };
        let r = vec![a, b];
        // Drive well into b's zoom-in window; the center should track b's anchor side.
        let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
        for t in (1000..1300).step_by(16) { c = s.step(t, FramePoint { x: 700, y: 500 }, &r, &cfg); }
        assert!(c.cx > 400.0, "expected to move toward b.anchor.x=700, got {}", c.cx);
    }

    #[test]
    fn center_clamps_inside_frame() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![ZoomRegion { anchor: FramePoint { x: 0, y: 0 }, ..region() }];
        let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
        for t in (0..1000).step_by(16) { c = s.step(t, FramePoint { x: 0, y: 0 }, &r, &cfg); }
        // at scale ~2 the half-view is 200x150; center must stay >= that
        assert!(c.cx >= 200.0 - 1.0 && c.cy >= 150.0 - 1.0);
    }
}
