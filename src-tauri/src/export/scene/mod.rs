use crate::actions::model::LayoutId;
use crate::export::coordmap::{corner_radius, full_src, inset_rect};
use crate::export::types::{Layout, OverlayLayout, OverlayPos, OverlayShape, RectF};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    pub rect: RectF,
    pub radius: f32,
    pub alpha: f32,
    pub ring_px: f32,
    pub ring_color: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene {
    pub screen: Panel,
    pub camera: Panel,
    pub src: RectF,
}

fn lf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn lr(a: RectF, b: RectF, t: f32) -> RectF {
    RectF {
        x: lf(a.x, b.x, t),
        y: lf(a.y, b.y, t),
        w: lf(a.w, b.w, t),
        h: lf(a.h, b.h, t),
    }
}
fn lp(a: Panel, b: Panel, t: f32) -> Panel {
    Panel {
        rect: lr(a.rect, b.rect, t),
        radius: lf(a.radius, b.radius, t),
        alpha: lf(a.alpha, b.alpha, t),
        ring_px: lf(a.ring_px, b.ring_px, t),
        ring_color: if t < 0.5 { a.ring_color } else { b.ring_color },
    }
}

impl Scene {
    pub fn lerp(a: &Scene, b: &Scene, t: f32) -> Scene {
        Scene {
            screen: lp(a.screen, b.screen, t),
            camera: lp(a.camera, b.camera, t),
            src: b.src,
        }
    }

    pub fn with_src(self, src: RectF) -> Scene {
        Scene { src, ..self }
    }
}

fn centered_square(l: &Layout, s: f32) -> RectF {
    RectF {
        x: (l.out_w as f32 - s) / 2.0,
        y: (l.out_h as f32 - s) / 2.0,
        w: s,
        h: s,
    }
}

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

fn panel_radius(shape: OverlayShape, w: f32, h: f32) -> f32 {
    match shape {
        OverlayShape::Circle => w.min(h) / 2.0,
        OverlayShape::Rounded { frac } => frac * w.min(h),
        OverlayShape::Rect => 0.0,
    }
}

pub fn resolve(id: LayoutId, layout: &Layout, overlay: &OverlayLayout, sw: u32, sh: u32) -> Scene {
    let (ow, oh, pad) = (
        layout.out_w as f32,
        layout.out_h as f32,
        layout.pad_px as f32,
    );
    let (ix, iy, iw, ih) = inset_rect(sw, sh, layout);
    let inset = RectF {
        x: ix as f32,
        y: iy as f32,
        w: iw as f32,
        h: ih as f32,
    };
    let inset_r = corner_radius(layout, iw, ih);
    let bubble = bubble_rect(overlay, ow, oh);
    let bubble_r = panel_radius(overlay.shape, bubble.w, bubble.h);
    let big = overlay.size_px as f32;
    let big_cam = centered_square(layout, big);
    let big_r = panel_radius(overlay.shape, big, big);
    let small_w = iw as f32 * 0.30;
    let small_h = small_w * sh.max(1) as f32 / sw.max(1) as f32;
    let small_screen = RectF {
        x: overlay.margin_x_px as f32,
        y: oh - small_h - overlay.margin_y_px as f32,
        w: small_w,
        h: small_h,
    };
    let small_r = corner_radius(layout, small_w as u32, small_h as u32);
    let pan = |rect, radius, alpha| Panel {
        rect,
        radius,
        alpha,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    };
    let cam_pan = |rect, radius, alpha| Panel {
        rect,
        radius,
        alpha,
        ring_px: overlay.ring_px as f32,
        ring_color: overlay.ring_color,
    };
    let src = full_src(sw, sh);
    match id {
        LayoutId::Screen => Scene {
            screen: pan(inset, inset_r, 1.0),
            camera: cam_pan(bubble, bubble_r, 1.0),
            src,
        },
        LayoutId::Camera => Scene {
            screen: pan(small_screen, small_r, 1.0),
            camera: cam_pan(big_cam, big_r, 1.0),
            src,
        },
        LayoutId::Presenter => {
            let gap = pad;
            let col = (ow - 2.0 * pad - gap) / 2.0;
            let avail_h = oh - 2.0 * pad;
            let cam_side = col.min(avail_h);
            let cam = RectF {
                x: pad,
                y: (oh - cam_side) / 2.0,
                w: cam_side,
                h: cam_side,
            };
            let sa = sw.max(1) as f32 / sh.max(1) as f32;
            let (rw, rh) = if col / avail_h > sa {
                (avail_h * sa, avail_h)
            } else {
                (col, col / sa)
            };
            let scr = RectF {
                x: pad + col + gap + (col - rw) / 2.0,
                y: (oh - rh) / 2.0,
                w: rw,
                h: rh,
            };
            Scene {
                screen: pan(scr, corner_radius(layout, rw as u32, rh as u32), 1.0),
                camera: cam_pan(cam, panel_radius(overlay.shape, cam_side, cam_side), 1.0),
                src,
            }
        }
        LayoutId::ScreenOnly => Scene {
            screen: pan(inset, inset_r, 1.0),
            camera: cam_pan(bubble, bubble_r, 0.0),
            src,
        },
        LayoutId::CameraOnly => Scene {
            screen: pan(inset, inset_r, 0.0),
            camera: cam_pan(big_cam, big_r, 1.0),
            src,
        },
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

mod cam;
pub use cam::{
    apply_cam_zoom_action, cam_action_at, override_camera, rect_from_center, shrink_camera,
};

pub mod arrangement;
pub mod background;
pub mod layout;
