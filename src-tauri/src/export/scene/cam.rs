use super::Panel;
use crate::export::camera::moves::CamPose;
use crate::export::types::{RectF, ZoomRegion};
use crate::settings::model::{CamZoomAction, ZoomSettings};

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn zoom_progress(scale: f32, target_scale: f32) -> f32 {
    ((scale - 1.0) / (target_scale - 1.0).max(0.001)).clamp(0.0, 1.0)
}

pub fn cam_action_at(
    regions: &[ZoomRegion],
    zoom: &ZoomSettings,
    out_t: u32,
) -> (CamZoomAction, f32) {
    let winner = regions
        .iter()
        .filter(|r| out_t >= r.start_ms && out_t <= r.end_ms)
        .max_by_key(|r| r.layer);
    let action = winner
        .and_then(|r| r.cam_action)
        .unwrap_or_else(|| zoom.resolved_cam_action());
    let target_scale = winner.map(|r| r.target_scale).unwrap_or(zoom.target_scale);
    (action, target_scale)
}

pub fn apply_cam_zoom_action(
    panel: Panel,
    action: CamZoomAction,
    scale: f32,
    target_scale: f32,
) -> Panel {
    match action {
        CamZoomAction::Shrink { to } => shrink_camera(panel, scale, target_scale, to),
        CamZoomAction::Hide => Panel {
            alpha: panel.alpha * (1.0 - smoothstep(zoom_progress(scale, target_scale))),
            ..panel
        },
        CamZoomAction::Stay => panel,
    }
}

pub fn shrink_camera(panel: Panel, scale: f32, target_scale: f32, min: f32) -> Panel {
    let z = zoom_progress(scale, target_scale);
    let m = 1.0 + (min.clamp(0.1, 1.0) - 1.0) * smoothstep(z);
    let (cx, cy) = (
        panel.rect.x + panel.rect.w / 2.0,
        panel.rect.y + panel.rect.h / 2.0,
    );
    let (w, h) = (panel.rect.w * m, panel.rect.h * m);
    Panel {
        rect: RectF {
            x: cx - w / 2.0,
            y: cy - h / 2.0,
            w,
            h,
        },
        radius: panel.radius * m,
        alpha: panel.alpha,
        ring_px: panel.ring_px * m,
        ring_color: panel.ring_color,
    }
}

pub fn rect_from_center(p: CamPose, ow: f32, oh: f32, aspect: f32) -> RectF {
    let h = p.size * oh;
    let w = h * aspect.max(0.01);
    RectF {
        x: p.x * ow - w / 2.0,
        y: p.y * oh - h / 2.0,
        w,
        h,
    }
}

pub fn override_camera(panel: Panel, p: CamPose, ow: f32, oh: f32, aspect: f32) -> Panel {
    let rect = rect_from_center(p, ow, oh, aspect);
    let m = rect.h / panel.rect.h.max(0.001);
    let radius = match p.round {
        Some(r) => r * rect.w.min(rect.h),
        None => panel.radius * m,
    };
    Panel {
        rect,
        radius,
        ring_px: panel.ring_px * m,
        ..panel
    }
}

#[cfg(test)]
#[path = "cam_tests.rs"]
mod cam_tests;

#[cfg(test)]
#[path = "action_tests.rs"]
mod action_tests;
