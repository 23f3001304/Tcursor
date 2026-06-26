use crate::export::easing::ease;
use crate::export::types::{Camera, FramePoint, ZoomConfig, ZoomRegion};

pub struct CameraSim { frame_w: u32, frame_h: u32, cx: f32, cy: f32 }

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self { frame_w, frame_h, cx: frame_w as f32 / 2.0, cy: frame_h as f32 / 2.0 }
    }

    pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera {
        let active = regions.iter().find(|r| t_ms >= r.start_ms && t_ms <= r.end_ms);
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        let (target_scale, target_cx, target_cy) = match active {
            None => (1.0, fw / 2.0, fh / 2.0),
            Some(r) => {
                let zin_end = r.start_ms + r.zoom_in_ms;
                let zout_start = r.end_ms.saturating_sub(r.zoom_out_ms);
                if t_ms < zin_end {
                    let p = (t_ms - r.start_ms) as f32 / r.zoom_in_ms.max(1) as f32;
                    (1.0 + (r.target_scale - 1.0) * ease(r.easing, p), r.anchor.x as f32, r.anchor.y as f32)
                } else if t_ms >= zout_start {
                    let p = (t_ms - zout_start) as f32 / r.zoom_out_ms.max(1) as f32;
                    (r.target_scale + (1.0 - r.target_scale) * ease(r.easing, p), fw / 2.0, fh / 2.0)
                } else {
                    // hold: follow cursor with a dead-zone around the current center
                    let mut tx = cursor.x as f32;
                    let mut ty = cursor.y as f32;
                    if (tx - self.cx).abs() < cfg.dead_zone_px as f32 { tx = self.cx; }
                    if (ty - self.cy).abs() < cfg.dead_zone_px as f32 { ty = self.cy; }
                    (r.target_scale, tx, ty)
                }
            }
        };
        // damped follow of the center
        self.cx += (target_cx - self.cx) * cfg.follow_damping.clamp(0.0, 1.0);
        self.cy += (target_cy - self.cy) * cfg.follow_damping.clamp(0.0, 1.0);
        // clamp so the view stays inside the frame
        let half_w = fw / (2.0 * target_scale);
        let half_h = fh / (2.0 * target_scale);
        self.cx = self.cx.clamp(half_w, fw - half_w);
        self.cy = self.cy.clamp(half_h, fh - half_h);
        Camera { cx: self.cx, cy: self.cy, scale: target_scale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{FramePoint, ZoomConfig, ZoomRegion, Easing};

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
    fn scale_eases_in_to_target_and_back_out() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![region()];
        let start = s.step(0, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
        let mid = { let mut x = 0.0; for t in (0..400).step_by(16) { x = s.step(t, FramePoint{x:400,y:300}, &r, &cfg).scale; } x };
        let endp = s.step(2000, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
        assert!(start < 1.5);          // easing in, not instantly at target
        assert!((mid - 2.0).abs() < 0.05); // reached target during hold
        assert!((endp - 1.0).abs() < 0.05); // eased back out by end
    }

    #[test]
    fn center_clamps_inside_frame() {
        let mut s = CameraSim::new(800, 600);
        let cfg = ZoomConfig::default();
        let r = vec![ZoomRegion { anchor: FramePoint { x: 0, y: 0 }, ..region() }];
        // hold phase, cursor at top-left corner
        let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
        for t in (0..1000).step_by(16) { c = s.step(t, FramePoint { x: 0, y: 0 }, &r, &cfg); }
        // at scale 2 the half-view is 200x150; center must stay >= that
        assert!(c.cx >= 200.0 - 1.0 && c.cy >= 150.0 - 1.0);
    }
}
