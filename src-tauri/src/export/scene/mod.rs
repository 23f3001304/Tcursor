use crate::actions::model::LayoutId;
use crate::export::camera::moves::CamPose;
use crate::export::coordmap::{corner_radius, inset_rect};
use crate::export::types::{Layout, OverlayLayout, OverlayPos, OverlayShape, RectF, ZoomRegion};
use crate::settings::model::{CamZoomAction, ZoomSettings};

/// One composited panel: a rounded rectangle (a circle is `radius = min(w,h)/2`)
/// with `alpha` in 0..1 for cross-dissolve (0 = absent). Both the screen panel and
/// the camera panel are `Panel`s, drawn with the same rounded-rect coverage.
/// `ring_px`/`ring_color` are only ever set on the camera panel (0/`[0,0,0]` = no ring).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel { pub rect: RectF, pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }

/// The two panels of a frame: the screen (zoomed base layer) + the camera (fixed top layer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene { pub screen: Panel, pub camera: Panel }

fn lf(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn lr(a: RectF, b: RectF, t: f32) -> RectF {
    RectF { x: lf(a.x, b.x, t), y: lf(a.y, b.y, t), w: lf(a.w, b.w, t), h: lf(a.h, b.h, t) }
}
fn lp(a: Panel, b: Panel, t: f32) -> Panel {
    // Ring color doesn't interpolate (RGB lerp mid-transition would look muddy); it snaps to `b`'s
    // color once the ring has any width, matching how `shape`/`pos` are also never blended - only
    // continuous numeric fields (rect/radius/alpha/ring width) cross-fade.
    Panel {
        rect: lr(a.rect, b.rect, t), radius: lf(a.radius, b.radius, t), alpha: lf(a.alpha, b.alpha, t),
        ring_px: lf(a.ring_px, b.ring_px, t), ring_color: if t < 0.5 { a.ring_color } else { b.ring_color },
    }
}

fn smoothstep(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

/// Zoom progress 0..1: 0 at `scale` 1.0, 1 at `target_scale`. Shared by every
/// `CamZoomAction` so the shrink and the fade ride the exact same curve.
fn zoom_progress(scale: f32, target_scale: f32) -> f32 {
    ((scale - 1.0) / (target_scale - 1.0).max(0.001)).clamp(0.0, 1.0)
}

/// The webcam action + its peak zoom scale in force at output time `out_t`: the highest-`layer`
/// zoom region containing `out_t` supplies both (the same overlap rule `CameraSim::step` uses to
/// pick a winner) - falling back to the global default action / `zoom.target_scale` when that
/// region inherits the action or no region is active. `target_scale` MUST be the winning region's
/// OWN `target_scale`, not the global one: `zoom_progress` (in `apply_cam_zoom_action`) divides by
/// it, so a region's shrink/hide only reaches its full effect at ITS OWN peak scale, not
/// `zoom.target_scale` - a 1.6x zoom fed the global 2.2 would only ever reach ~50% progress.
pub fn cam_action_at(regions: &[ZoomRegion], zoom: &ZoomSettings, out_t: u32) -> (CamZoomAction, f32) {
    let winner = regions.iter()
        .filter(|r| out_t >= r.start_ms && out_t <= r.end_ms)
        .max_by_key(|r| r.layer);
    let action = winner.and_then(|r| r.cam_action).unwrap_or_else(|| zoom.resolved_cam_action());
    let target_scale = winner.map(|r| r.target_scale).unwrap_or(zoom.target_scale);
    (action, target_scale)
}

/// Apply the resolved webcam-on-zoom action to the camera panel. `Shrink` delegates straight
/// to `shrink_camera`, so a default-resolved action reproduces the pre-action behavior exactly;
/// `Hide` fades the panel out on the same smoothstepped progress; `Stay` is the identity.
pub fn apply_cam_zoom_action(panel: Panel, action: CamZoomAction, scale: f32, target_scale: f32) -> Panel {
    match action {
        CamZoomAction::Shrink { to } => shrink_camera(panel, scale, target_scale, to),
        CamZoomAction::Hide => Panel {
            alpha: panel.alpha * (1.0 - smoothstep(zoom_progress(scale, target_scale))), ..panel
        },
        CamZoomAction::Stay => panel,
    }
}

/// Shrink the camera panel toward its center by a factor driven by the zoom scale:
/// full size at `scale` 1.0, down to `min` at `target_scale` (smoothstepped), so the
/// webcam stays out of the way during zoom-in and returns on zoom-out.
pub fn shrink_camera(panel: Panel, scale: f32, target_scale: f32, min: f32) -> Panel {
    let z = zoom_progress(scale, target_scale);
    let m = 1.0 + (min.clamp(0.1, 1.0) - 1.0) * smoothstep(z);
    let (cx, cy) = (panel.rect.x + panel.rect.w / 2.0, panel.rect.y + panel.rect.h / 2.0);
    let (w, h) = (panel.rect.w * m, panel.rect.h * m);
    Panel {
        rect: RectF { x: cx - w / 2.0, y: cy - h / 2.0, w, h }, radius: panel.radius * m, alpha: panel.alpha,
        ring_px: panel.ring_px * m, ring_color: panel.ring_color,
    }
}

/// Convert a sampled `CamPose` (center x/y + height, all fractions of the output frame) into
/// the camera panel's `RectF` (top-left form). `aspect` is the panel's own w/h, so a Wide
/// (16:9) panel keeps its shape - the pose only ever carries height.
pub fn rect_from_center(p: CamPose, ow: f32, oh: f32, aspect: f32) -> RectF {
    let h = p.size * oh;
    let w = h * aspect.max(0.01);
    RectF { x: p.x * ow - w / 2.0, y: p.y * oh - h / 2.0, w, h }
}

/// Apply a `camera_moves` keyframe override to `panel`: replace its rect with the sampled
/// pose (via `rect_from_center`) AND scale `radius` by the height ratio, so a circle
/// (`radius == min(w,h)/2` at the static size) stays a true circle after a keyframe
/// grows/shrinks the panel instead of distorting toward the STATIC radius. Ring width
/// scales the same way (matches `shrink_camera`'s treatment), for the same reason.
pub fn override_camera(panel: Panel, p: CamPose, ow: f32, oh: f32, aspect: f32) -> Panel {
    let old_h = panel.rect.h.max(0.001);
    let rect = rect_from_center(p, ow, oh, aspect);
    let m = rect.h / old_h;
    Panel { rect, radius: panel.radius * m, ring_px: panel.ring_px * m, ..panel }
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
/// Width comes from `ov.width_px` (== `size_px` unless `cam_aspect` is Wide) so the
/// panel can be wider than tall without affecting the height-driven `cam_size`.
fn bubble_rect(ov: &OverlayLayout, ow: f32, oh: f32) -> RectF {
    let (w, h) = (ov.width_px as f32, ov.size_px as f32);
    let (mx, my) = (ov.margin_x_px as f32, ov.margin_y_px as f32);
    let (x, y) = match ov.pos {
        OverlayPos::BottomLeft => (mx, oh - h - my),
        OverlayPos::BottomRight => (ow - w - mx, oh - h - my),
        OverlayPos::TopLeft => (mx, my),
        OverlayPos::TopRight => (ow - w - mx, my),
        OverlayPos::Custom { x, y } => (x as f32, y as f32),
    };
    RectF { x, y, w, h }
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
    // Screen panel never has a ring; camera panel carries `overlay.ring_px`/`ring_color` (0/black
    // when `cam_ring` is unset, so the shader's `ring.x > 0.0` gate stays closed by default).
    let pan = |rect, radius, alpha| Panel { rect, radius, alpha, ring_px: 0.0, ring_color: [0, 0, 0] };
    let cam_pan = |rect, radius, alpha| Panel { rect, radius, alpha, ring_px: overlay.ring_px as f32, ring_color: overlay.ring_color };
    match id {
        LayoutId::Screen => Scene { screen: pan(inset, inset_r, 1.0), camera: cam_pan(bubble, bubble_r, 1.0) },
        LayoutId::Camera => Scene { screen: pan(small_screen, small_r, 1.0), camera: cam_pan(big_cam, big_r, 1.0) },
        LayoutId::Presenter => {
            let gap = pad;
            let col = (ow - 2.0 * pad - gap) / 2.0;
            let avail_h = oh - 2.0 * pad;
            let cam_side = col.min(avail_h);
            let cam = RectF { x: pad, y: (oh - cam_side) / 2.0, w: cam_side, h: cam_side };
            let sa = sw.max(1) as f32 / sh.max(1) as f32;
            let (rw, rh) = if col / avail_h > sa { (avail_h * sa, avail_h) } else { (col, col / sa) };
            let scr = RectF { x: pad + col + gap + (col - rw) / 2.0, y: (oh - rh) / 2.0, w: rw, h: rh };
            Scene { screen: pan(scr, corner_radius(layout, rw as u32, rh as u32), 1.0), camera: cam_pan(cam, panel_radius(overlay.shape, cam_side, cam_side), 1.0) }
        }
        LayoutId::ScreenOnly => Scene { screen: pan(inset, inset_r, 1.0), camera: cam_pan(bubble, bubble_r, 0.0) },
        LayoutId::CameraOnly => Scene { screen: pan(inset, inset_r, 0.0), camera: cam_pan(big_cam, big_r, 1.0) },
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cam_tests.rs"]
mod cam_tests;

#[cfg(test)]
#[path = "action_tests.rs"]
mod action_tests;

pub mod layout;
pub mod background;
pub mod arrangement;
