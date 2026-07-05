use crate::actions::model::LayoutId;
use crate::export::coordmap::{corner_radius, inset_rect};
use crate::export::types::{Layout, OverlayLayout, OverlayPos, OverlayShape, RectF};

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

fn smoothstep(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

/// Shrink the camera panel toward its center by a factor driven by the zoom scale:
/// full size at `scale` 1.0, down to `min` at `target_scale` (smoothstepped), so the
/// webcam stays out of the way during zoom-in and returns on zoom-out.
pub fn shrink_camera(panel: Panel, scale: f32, target_scale: f32, min: f32) -> Panel {
    let z = ((scale - 1.0) / (target_scale - 1.0).max(0.001)).clamp(0.0, 1.0);
    let m = 1.0 + (min.clamp(0.1, 1.0) - 1.0) * smoothstep(z);
    let (cx, cy) = (panel.rect.x + panel.rect.w / 2.0, panel.rect.y + panel.rect.h / 2.0);
    let (w, h) = (panel.rect.w * m, panel.rect.h * m);
    Panel { rect: RectF { x: cx - w / 2.0, y: cy - h / 2.0, w, h }, radius: panel.radius * m, alpha: panel.alpha }
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

/// The webcam bubble rect for a corner position + per-axis margins (output px).
fn bubble_rect(ov: &OverlayLayout, ow: f32, oh: f32) -> RectF {
    let s = ov.size_px as f32;
    let (mx, my) = (ov.margin_x_px as f32, ov.margin_y_px as f32);
    let (x, y) = match ov.pos {
        OverlayPos::BottomLeft => (mx, oh - s - my),
        OverlayPos::BottomRight => (ow - s - mx, oh - s - my),
        OverlayPos::TopLeft => (mx, my),
        OverlayPos::TopRight => (ow - s - mx, my),
        OverlayPos::Custom { x, y } => (x as f32, y as f32),
    };
    RectF { x, y, w: s, h: s }
}

/// Corner radius for a camera panel of size `w` x `h` given its shape.
fn panel_radius(shape: OverlayShape, w: f32, h: f32) -> f32 {
    match shape {
        OverlayShape::Circle => w.min(h) / 2.0,
        OverlayShape::Rounded { frac } => frac * w.min(h),
        OverlayShape::Rect => 0.0,
    }
}

/// Resolve a preset into its two panels (output pixels). `overlay` supplies the
/// webcam look (shape/pos/size/margins) and `layout` the screen scale/radius.
/// Disabled panels keep a sensible rect so cross-dissolve transitions stay smooth.
pub fn resolve(id: LayoutId, layout: &Layout, overlay: &OverlayLayout, sw: u32, sh: u32) -> Scene {
    let (ow, oh, pad) = (layout.out_w as f32, layout.out_h as f32, layout.pad_px as f32);
    let (ix, iy, iw, ih) = inset_rect(sw, sh, layout);
    let inset = RectF { x: ix as f32, y: iy as f32, w: iw as f32, h: ih as f32 };
    let inset_r = corner_radius(layout, iw, ih);
    let bubble = bubble_rect(overlay, ow, oh);
    let bubble_r = panel_radius(overlay.shape, bubble.w, bubble.h);
    let big = overlay.size_px as f32;                  // big-camera square side (settings-driven)
    let big_cam = centered_square(layout, big);
    let big_r = panel_radius(overlay.shape, big, big);
    let small_w = iw as f32 * 0.30;
    let small_h = small_w * sh.max(1) as f32 / sw.max(1) as f32;
    let small_screen = RectF { x: overlay.margin_x_px as f32, y: oh - small_h - overlay.margin_y_px as f32, w: small_w, h: small_h };
    let small_r = corner_radius(layout, small_w as u32, small_h as u32);
    let pan = |rect, radius, alpha| Panel { rect, radius, alpha };
    match id {
        LayoutId::Screen => Scene { screen: pan(inset, inset_r, 1.0), camera: pan(bubble, bubble_r, 1.0) },
        LayoutId::Camera => Scene { screen: pan(small_screen, small_r, 1.0), camera: pan(big_cam, big_r, 1.0) },
        LayoutId::Presenter => {
            let gap = pad;
            let col = (ow - 2.0 * pad - gap) / 2.0;
            let avail_h = oh - 2.0 * pad;
            let cam_side = col.min(avail_h);
            let cam = RectF { x: pad, y: (oh - cam_side) / 2.0, w: cam_side, h: cam_side };
            let sa = sw.max(1) as f32 / sh.max(1) as f32;
            let (rw, rh) = if col / avail_h > sa { (avail_h * sa, avail_h) } else { (col, col / sa) };
            let scr = RectF { x: pad + col + gap + (col - rw) / 2.0, y: (oh - rh) / 2.0, w: rw, h: rh };
            Scene { screen: pan(scr, corner_radius(layout, rw as u32, rh as u32), 1.0), camera: pan(cam, panel_radius(overlay.shape, cam_side, cam_side), 1.0) }
        }
        LayoutId::ScreenOnly => Scene { screen: pan(inset, inset_r, 1.0), camera: pan(bubble, bubble_r, 0.0) },
        LayoutId::CameraOnly => Scene { screen: pan(inset, inset_r, 0.0), camera: pan(big_cam, big_r, 1.0) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::coordmap::inset_rect;
    use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings, CamCorner, CamShape};

    fn mode(id: LayoutId, sw: u32, sh: u32) -> Scene {
        let a = AppearanceSettings::default();
        let ma = a.for_id(id);
        resolve(id, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), sw, sh)
    }

    #[test]
    fn screen_default_matches_today_inset_and_bubble() {
        let s = mode(LayoutId::Screen, 1920, 1080);
        let lay = layout_for(&AppearanceSettings::default().screen, 3840, 2160);
        let (ix, iy, iw, ih) = inset_rect(1920, 1080, &lay);
        assert_eq!(s.screen.rect, RectF { x: ix as f32, y: iy as f32, w: iw as f32, h: ih as f32 });
        assert_eq!(s.camera.rect.w, 420.0);
        assert_eq!(s.camera.rect.x, 80.0);
        assert!((s.camera.rect.y - (2160.0 - 420.0 - 80.0)).abs() < 1.0);
        assert!((s.camera.radius - 210.0).abs() < 1e-3);   // circle
        assert_eq!((s.screen.alpha, s.camera.alpha), (1.0, 1.0));
    }
    #[test]
    fn camera_default_is_big_centered_rounded_square() {
        let s = mode(LayoutId::Camera, 1920, 1080);
        assert!((s.camera.rect.w - 1920.0).abs() < 1.0);
        assert!((s.camera.rect.x - (3840.0 - 1920.0) / 2.0).abs() < 1.0);
        assert!((s.camera.radius - 0.04 * 1920.0).abs() < 1.0); // 76.8
        assert!(s.camera.rect.w * s.camera.rect.h > s.screen.rect.w * s.screen.rect.h);
    }
    #[test]
    fn corner_and_shape_knobs_apply() {
        let mut a = AppearanceSettings::default();
        a.screen.cam_corner = CamCorner::TopRight;
        a.screen.cam_shape = CamShape::Rect;
        let ma = a.for_id(LayoutId::Screen);
        let s = resolve(LayoutId::Screen, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), 1920, 1080);
        assert_eq!(s.camera.rect.x, 3840.0 - 420.0 - 80.0); // right edge
        assert_eq!(s.camera.rect.y, 80.0);                  // top edge
        assert_eq!(s.camera.radius, 0.0);                   // rect -> no rounding
    }
    #[test]
    fn camera_only_disables_screen() {
        let s = mode(LayoutId::CameraOnly, 1920, 1080);
        assert_eq!(s.screen.alpha, 0.0);
        assert_eq!(s.camera.alpha, 1.0);
    }
    #[test]
    fn camera_default_pip_screen_keeps_today_inset() {
        let s = mode(LayoutId::Camera, 1920, 1080);
        // PiP screen stays inset 80px from the bottom-left (today's behavior), not flush.
        assert!((s.screen.rect.x - 80.0).abs() < 1.0);
        let small_h = s.screen.rect.h;
        assert!((s.screen.rect.y - (2160.0 - small_h - 80.0)).abs() < 1.0);
    }
    #[test]
    fn lerp_midpoint_is_between() {
        let a = mode(LayoutId::Screen, 1920, 1080);
        let b = mode(LayoutId::Camera, 1920, 1080);
        let m = Scene::lerp(&a, &b, 0.5);
        assert!((m.screen.rect.w - (a.screen.rect.w + b.screen.rect.w) / 2.0).abs() < 1e-3);
    }
    #[test]
    fn shrink_is_identity_at_no_zoom_and_min_at_full() {
        let p = Panel { rect: RectF { x: 100.0, y: 100.0, w: 200.0, h: 200.0 }, radius: 100.0, alpha: 1.0 };
        let none = shrink_camera(p, 1.0, 2.2, 0.6);
        assert_eq!(none.rect.w, 200.0);
        assert_eq!((none.rect.x, none.rect.y), (100.0, 100.0));
        let full = shrink_camera(p, 2.2, 2.2, 0.6);
        assert!((full.rect.w - 120.0).abs() < 0.5);
        assert!((full.radius - 60.0).abs() < 0.5);
        assert!((full.rect.x + full.rect.w / 2.0 - 200.0).abs() < 0.5);
    }
}

pub mod layout;
pub mod background;
