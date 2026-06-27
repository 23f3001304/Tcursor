use crate::actions::model::LayoutId;
use crate::export::coordmap::{corner_radius, inset_rect};
use crate::export::types::{Layout, OverlayLayout, RectF};

/// One composited panel: a rounded rectangle (a circle is `radius = min(w,h)/2`)
/// with `alpha` in 0..1 for cross-dissolve (0 = absent). Both the screen panel and
/// the camera panel are `Panel`s, drawn with the same rounded-rect coverage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel { pub rect: RectF, pub radius: f32, pub alpha: f32 }

/// The two panels of a frame: the screen (zoomed base layer) + the camera (fixed top layer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene { pub screen: Panel, pub camera: Panel }

fn lf(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn lr(a: RectF, b: RectF, t: f32) -> RectF {
    RectF { x: lf(a.x, b.x, t), y: lf(a.y, b.y, t), w: lf(a.w, b.w, t), h: lf(a.h, b.h, t) }
}
fn lp(a: Panel, b: Panel, t: f32) -> Panel {
    Panel { rect: lr(a.rect, b.rect, t), radius: lf(a.radius, b.radius, t), alpha: lf(a.alpha, b.alpha, t) }
}

impl Scene {
    /// Component-wise interpolation between two scenes (eased `t` supplied by the caller).
    pub fn lerp(a: &Scene, b: &Scene, t: f32) -> Scene {
        Scene { screen: lp(a.screen, b.screen, t), camera: lp(a.camera, b.camera, t) }
    }
}

fn centered_square(layout: &Layout, s: f32) -> RectF {
    RectF { x: (layout.out_w as f32 - s) / 2.0, y: (layout.out_h as f32 - s) / 2.0, w: s, h: s }
}

/// Resolve a preset into its two panels (output pixels). `overlay` supplies the
/// `Screen`-preset camera-bubble look (size/margin) so `Screen` matches today.
/// Disabled panels keep a sensible rect (their related-preset placement) so a
/// transition that toggles a panel cross-dissolves in place rather than popping.
pub fn resolve(id: LayoutId, layout: &Layout, overlay: &OverlayLayout, sw: u32, sh: u32) -> Scene {
    let (ow, oh, pad) = (layout.out_w as f32, layout.out_h as f32, layout.pad_px as f32);
    let (ix, iy, iw, ih) = inset_rect(sw, sh, layout);
    let inset = RectF { x: ix as f32, y: iy as f32, w: iw as f32, h: ih as f32 };
    let inset_r = corner_radius(layout, iw, ih);
    let bsz = overlay.size_px as f32;
    let bm = overlay.margin_px as f32;
    let bubble = RectF { x: bm, y: oh - bsz - bm, w: bsz, h: bsz };
    let big = (ow - 2.0 * pad).min(oh - 2.0 * pad);
    let big_cam = centered_square(layout, big);
    let small_w = iw as f32 * 0.30;
    let small_h = small_w * sh.max(1) as f32 / sw.max(1) as f32;
    let small_screen = RectF { x: bm, y: oh - small_h - bm, w: small_w, h: small_h };
    let small_r = corner_radius(layout, small_w as u32, small_h as u32);
    let pan = |rect, radius, alpha| Panel { rect, radius, alpha };
    match id {
        LayoutId::Screen => Scene { screen: pan(inset, inset_r, 1.0), camera: pan(bubble, bsz / 2.0, 1.0) },
        LayoutId::Camera => Scene { screen: pan(small_screen, small_r, 1.0), camera: pan(big_cam, big * 0.04, 1.0) },
        LayoutId::Presenter => {
            let gap = pad;
            let col = (ow - 2.0 * pad - gap) / 2.0;
            let avail_h = oh - 2.0 * pad;
            let cam_side = col.min(avail_h);
            let cam = RectF { x: pad, y: (oh - cam_side) / 2.0, w: cam_side, h: cam_side };
            let sa = sw.max(1) as f32 / sh.max(1) as f32;
            let (rw, rh) = if col / avail_h > sa { (avail_h * sa, avail_h) } else { (col, col / sa) };
            let scr = RectF { x: pad + col + gap + (col - rw) / 2.0, y: (oh - rh) / 2.0, w: rw, h: rh };
            Scene { screen: pan(scr, corner_radius(layout, rw as u32, rh as u32), 1.0), camera: pan(cam, cam_side * 0.04, 1.0) }
        }
        LayoutId::ScreenOnly => Scene { screen: pan(inset, inset_r, 1.0), camera: pan(bubble, bsz / 2.0, 0.0) },
        LayoutId::CameraOnly => Scene { screen: pan(inset, inset_r, 0.0), camera: pan(big_cam, big * 0.04, 1.0) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::coordmap::inset_rect;

    #[test]
    fn screenfocus_matches_today_inset_and_bubble() {
        let layout = Layout { out_w: 1920, out_h: 1080, pad_px: 60 };
        let ov = OverlayLayout::default();
        let s = resolve(LayoutId::Screen, &layout, &ov, 1920, 1080);
        let (ix, iy, iw, ih) = inset_rect(1920, 1080, &layout);
        assert_eq!(s.screen.rect, RectF { x: ix as f32, y: iy as f32, w: iw as f32, h: ih as f32 });
        assert_eq!(s.screen.alpha, 1.0);
        assert_eq!(s.camera.rect.w, ov.size_px as f32);
        assert!((s.camera.radius - ov.size_px as f32 / 2.0).abs() < 1e-3); // circle
        assert_eq!(s.camera.alpha, 1.0);
    }

    #[test]
    fn camera_only_disables_screen() {
        let s = resolve(LayoutId::CameraOnly, &Layout::default(), &OverlayLayout::default(), 1920, 1080);
        assert_eq!(s.screen.alpha, 0.0);
        assert_eq!(s.camera.alpha, 1.0);
    }

    #[test]
    fn camerafocus_camera_dominates_screen() {
        let s = resolve(LayoutId::Camera, &Layout::default(), &OverlayLayout::default(), 1920, 1080);
        assert!(s.camera.rect.w * s.camera.rect.h > s.screen.rect.w * s.screen.rect.h);
    }

    #[test]
    fn lerp_midpoint_is_between() {
        let (l, ov) = (Layout::default(), OverlayLayout::default());
        let a = resolve(LayoutId::Screen, &l, &ov, 1920, 1080);
        let b = resolve(LayoutId::Camera, &l, &ov, 1920, 1080);
        let m = Scene::lerp(&a, &b, 0.5);
        assert!((m.screen.rect.w - (a.screen.rect.w + b.screen.rect.w) / 2.0).abs() < 1e-3);
        assert!((m.camera.alpha - 1.0).abs() < 1e-6);
    }
}
