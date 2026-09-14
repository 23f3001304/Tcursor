//! Pose-based panel arrangements (T34): resolve a `LayoutSeg.arrangement` into the very same
//! `Scene` shape the five presets resolve to, and derive an `Arrangement` back OUT of a resolved
//! preset so a preset can seed one. Both directions go through the center+aspect machinery the
//! camera keyframes already use (`rect_from_center`/`override_camera`), which is what makes
//! `resolve_arrangement(arrangement_of_preset(p)) == resolve(p)` hold to the pixel.
use crate::edit::model::{Arrangement, PanelPose};
use crate::export::camera::moves::CamPose;
use crate::export::coordmap::corner_radius;
use crate::export::scene::{override_camera, rect_from_center, Panel, Scene};
use crate::export::types::{Layout, OverlayLayout};

/// A panel at or below this alpha is "not shown", so `arrangement_of_preset` maps it to `None`.
/// Matches the preview's own draw threshold (`toPreviewLayout` in editor/timeline/layoutTrack.ts);
/// a preset-resolved panel is only ever exactly 0.0 or 1.0 anyway.
const SHOWN_ALPHA: f32 = 0.004;

/// An arrangement pose carries no shape of its own (`round: None`): the preset's radius scales
/// with the resize, exactly as it did before keyframes could carry a shape.
fn cam_pose(p: PanelPose) -> CamPose { CamPose { x: p.cx, y: p.cy, size: p.size, round: None } }

/// The screen panel's own aspect - the CAPTURED screen's, because every preset aspect-fits the
/// source into its screen rect (`inset_rect`, and the derived small/presenter rects alike).
fn screen_aspect(sw: u32, sh: u32) -> f32 { sw.max(1) as f32 / sh.max(1) as f32 }

/// The webcam panel's own aspect, straight off the appearance-configured shape: `width_px` is
/// `size_px` unless `cam_aspect` is `Wide`, in which case the panel is 16:9 (R3/T14).
fn cam_aspect(ov: &OverlayLayout) -> f32 { ov.width_px.max(1) as f32 / ov.size_px.max(1) as f32 }

/// One resolved panel's pose, or `None` when the panel is not shown. `size` is the HEIGHT
/// fraction: the width is never stored, so it can only ever be re-derived from the panel's aspect.
pub fn pose_of_panel(p: &Panel, ow: f32, oh: f32) -> Option<PanelPose> {
    (p.alpha > SHOWN_ALPHA).then(|| PanelPose {
        cx: (p.rect.x + p.rect.w / 2.0) / ow.max(1.0),
        cy: (p.rect.y + p.rect.h / 2.0) / oh.max(1.0),
        size: p.rect.h / oh.max(1.0),
    })
}

/// The arrangement equivalent to an already-resolved preset `Scene` - the "preset as a one-click
/// starting point" conversion. Hidden panels (`ScreenOnly`'s cam, `CameraOnly`'s screen) become
/// `None`, so a converted preset shows exactly what the preset showed.
pub fn arrangement_of_preset(s: &Scene, ow: f32, oh: f32) -> Arrangement {
    Arrangement { screen: pose_of_panel(&s.screen, ow, oh), cam: pose_of_panel(&s.camera, ow, oh) }
}

/// Resolve an arrangement into a `Scene`. `base` is the segment's PRESET-resolved scene (from its
/// `layout` name, which still selects the appearance block): a posed panel takes its rect from the
/// pose and everything else - the screen's radius rule, the cam's shape/radius/ring - from exactly
/// where the preset got it, and a `None` panel keeps the preset's rect at `alpha = 0` so a
/// cross-dissolve to or from it still slides rather than popping.
pub fn resolve_arrangement(a: &Arrangement, base: Scene, layout: &Layout, ov: &OverlayLayout,
                           sw: u32, sh: u32) -> Scene {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let screen = match a.screen {
        None => Panel { alpha: 0.0, ..base.screen },
        Some(p) => {
            let rect = rect_from_center(cam_pose(p), ow, oh, screen_aspect(sw, sh));
            Panel { rect, radius: corner_radius(layout, rect.w.max(0.0) as u32, rect.h.max(0.0) as u32),
                alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] }
        }
    };
    let camera = match a.cam {
        None => Panel { alpha: 0.0, ..base.camera },
        // `override_camera` is the keyframe path verbatim: it re-centers the rect and scales the
        // static radius + ring by the height ratio, so a circle stays a circle and the ring keeps
        // its proportion at any pose (and is the IDENTITY when the pose reproduces the preset).
        Some(p) => Panel { alpha: 1.0, ..override_camera(base.camera, cam_pose(p), ow, oh, cam_aspect(ov)) },
    };
    // `src` rides through from the preset scene: an arrangement poses PANELS, never what the screen
    // panel is showing, so a posed segment on a switched-to display keeps that span's crop rect.
    Scene { screen, camera, src: base.src }
}

#[cfg(test)]
#[path = "arrangement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "arrangement_multi_res_tests.rs"]
mod multi_res_tests;
