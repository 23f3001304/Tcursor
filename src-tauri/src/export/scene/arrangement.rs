use crate::edit::model::{Arrangement, PanelPose};
use crate::export::camera::moves::CamPose;
use crate::export::coordmap::corner_radius;
use crate::export::scene::{override_camera, rect_from_center, Panel, Scene};
use crate::export::types::{Layout, OverlayLayout};

const SHOWN_ALPHA: f32 = 0.004;

fn cam_pose(p: PanelPose) -> CamPose {
    CamPose {
        x: p.cx,
        y: p.cy,
        size: p.size,
        round: None,
    }
}

fn screen_aspect(sw: u32, sh: u32) -> f32 {
    sw.max(1) as f32 / sh.max(1) as f32
}

fn cam_aspect(ov: &OverlayLayout) -> f32 {
    ov.width_px.max(1) as f32 / ov.size_px.max(1) as f32
}

pub fn pose_of_panel(p: &Panel, ow: f32, oh: f32) -> Option<PanelPose> {
    (p.alpha > SHOWN_ALPHA).then(|| PanelPose {
        cx: (p.rect.x + p.rect.w / 2.0) / ow.max(1.0),
        cy: (p.rect.y + p.rect.h / 2.0) / oh.max(1.0),
        size: p.rect.h / oh.max(1.0),
    })
}

pub fn arrangement_of_preset(s: &Scene, ow: f32, oh: f32) -> Arrangement {
    Arrangement {
        screen: pose_of_panel(&s.screen, ow, oh),
        cam: pose_of_panel(&s.camera, ow, oh),
    }
}

pub fn resolve_arrangement(
    a: &Arrangement,
    base: Scene,
    layout: &Layout,
    ov: &OverlayLayout,
    sw: u32,
    sh: u32,
) -> Scene {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let screen = match a.screen {
        None => Panel {
            alpha: 0.0,
            ..base.screen
        },
        Some(p) => {
            let rect = rect_from_center(cam_pose(p), ow, oh, screen_aspect(sw, sh));
            Panel {
                rect,
                radius: corner_radius(layout, rect.w.max(0.0) as u32, rect.h.max(0.0) as u32),
                alpha: 1.0,
                ring_px: 0.0,
                ring_color: [0, 0, 0],
            }
        }
    };
    let camera = match a.cam {
        None => Panel {
            alpha: 0.0,
            ..base.camera
        },
        Some(p) => Panel {
            alpha: 1.0,
            ..override_camera(base.camera, cam_pose(p), ow, oh, cam_aspect(ov))
        },
    };
    Scene {
        screen,
        camera,
        src: base.src,
    }
}

#[cfg(test)]
#[path = "arrangement_tests.rs"]
mod tests;
