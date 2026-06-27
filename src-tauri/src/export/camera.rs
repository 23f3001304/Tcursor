use crate::export::types::{Camera, FramePoint, ZoomConfig, ZoomRegion};

/// A virtual camera that smoothly damps its zoom and center toward a per-frame
/// setpoint derived from the active zoom region. The most-recent active region
/// wins, so a new click preempts an older region's zoom-out instead of fighting
/// it; scale and center are eased by exponential damping rather than recomputed
/// absolutely, which keeps motion continuous across click transitions.
pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32 }

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self { frame_w, frame_h, cx: frame_w as f32 / 2.0, cy: frame_h as f32 / 2.0, scale: 1.0 }
    }

    pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera {
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        // Most-recent active region wins so a fresh click takes over immediately.
        let active = regions.iter().rev().find(|r| t_ms >= r.start_ms && t_ms <= r.end_ms);
        let (target_scale, target_cx, target_cy) = match active {
            None => (1.0, fw / 2.0, fh / 2.0),
            Some(r) => {
                let zin_end = r.start_ms + r.zoom_in_ms;
                let zout_start = r.end_ms.saturating_sub(r.zoom_out_ms);
                if t_ms < zin_end {
                    // zoom in: home to the click point
                    (r.target_scale, r.anchor.x as f32, r.anchor.y as f32)
                } else if t_ms >= zout_start {
                    // release: un-zoom in place; the clamp recenters as scale -> 1
                    (1.0, self.cx, self.cy)
                } else {
                    // hold: keep the cursor inside the zoomed viewport but stay put
                    // while it roams the central region — pan only when it nears an
                    // edge. Steady hold + smooth pan on big moves (no jitter chase).
                    let mx = fw / (2.0 * r.target_scale) * 0.4;
                    let my = fh / (2.0 * r.target_scale) * 0.4;
                    let dx = cursor.x as f32 - self.cx;
                    let dy = cursor.y as f32 - self.cy;
                    let tx = if dx > mx { cursor.x as f32 - mx }
                        else if dx < -mx { cursor.x as f32 + mx } else { self.cx };
                    let ty = if dy > my { cursor.y as f32 - my }
                        else if dy < -my { cursor.y as f32 + my } else { self.cy };
                    (r.target_scale, tx, ty)
                }
            }
        };
        // Exponential damping toward the setpoint. Scale and center use the SAME
        // rate so the zoom-in and the pan move in lockstep — one coordinated push
        // toward the point, instead of an awkward scale-then-pan (or pan-then-scale).
        let k = cfg.follow_damping.clamp(0.0, 1.0);
        let ks = k;
        self.scale += (target_scale - self.scale) * ks;
        self.cx += (target_cx - self.cx) * k;
        self.cy += (target_cy - self.cy) * k;
        // Clamp the center so the view (frame / current scale) stays in-frame.
        let half_w = fw / (2.0 * self.scale.max(0.01));
        let half_h = fh / (2.0 * self.scale.max(0.01));
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
    fn scale_damps_in_to_target_then_back_out() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![region()];
        let start = s.step(0, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
        let mid = { let mut x = 0.0; for t in (0..600).step_by(16) { x = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg).scale; } x };
        let endp = { let mut x = 0.0; for t in (1700..2400).step_by(16) { x = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg).scale; } x };
        assert!(start < 1.5);              // damps in, not instantly at target
        assert!((mid - 2.0).abs() < 0.05); // reached target during hold
        assert!((endp - 1.0).abs() < 0.1); // damped back out by release end
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
