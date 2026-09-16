use crate::edit::effect::{
    DEFAULT_BLUR, DEFAULT_MASK_FEATHER, DEFAULT_MASK_ROUNDNESS, DEFAULT_PIXEL,
};
use crate::edit::model::{EffectKind, EffectRegion};
use crate::export::coordmap::{project, to_panel};
use crate::export::fx::spot::spotlight_sim::region_alpha;
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint, RectF};

pub const HIDDEN_PANEL_ALPHA: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskDraw {
    pub mn: [f32; 2],
    pub mx: [f32; 2],
    pub r: f32,
    pub feather_px: f32,
    pub amount_px: f32,
    pub dim: f32,
    pub kind: u32,
    pub alpha: f32,
}

pub fn mask_kind_id(kind: EffectKind) -> u32 {
    match kind {
        EffectKind::Spotlight => 0,
        EffectKind::Blur => 1,
        EffectKind::Pixelate => 2,
        EffectKind::Highlight => 3,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn masks_at(
    effects: &[EffectRegion],
    scene: &Scene,
    cam: Camera,
    src_full: RectF,
    ow: u32,
    oh: u32,
    region_t: u32,
    default_dim: f32,
) -> Vec<MaskDraw> {
    if scene.screen.alpha < HIDDEN_PANEL_ALPHA {
        return Vec::new();
    }
    let mut live: Vec<(u32, usize, &EffectRegion)> = effects
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind.is_mask())
        .map(|(i, e)| (e.layer, i, e))
        .collect();
    live.sort_by_key(|(layer, i, _)| (*layer, *i));
    live.into_iter()
        .filter_map(|(_, _, e)| one(e, scene, cam, src_full, ow, oh, region_t, default_dim))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn one(
    e: &EffectRegion,
    scene: &Scene,
    cam: Camera,
    src_full: RectF,
    ow: u32,
    oh: u32,
    region_t: u32,
    default_dim: f32,
) -> Option<MaskDraw> {
    let alpha = region_alpha(e, region_t);
    if alpha <= 0.0 {
        return None;
    }
    let (mn, mx) = project_rect(e.rect?, scene, cam, src_full, ow, oh)?;
    let short = (mx[0] - mn[0]).min(mx[1] - mn[1]);
    let amount = e.strength.unwrap_or(match e.kind {
        EffectKind::Pixelate => DEFAULT_PIXEL,
        _ => DEFAULT_BLUR,
    });
    Some(MaskDraw {
        mn,
        mx,
        r: short
            * e.roundness
                .unwrap_or(DEFAULT_MASK_ROUNDNESS)
                .clamp(0.0, 0.5),
        feather_px: oh as f32 * e.feather.unwrap_or(DEFAULT_MASK_FEATHER).max(0.0),
        amount_px: oh as f32 * amount.max(0.0),
        dim: e.dim.unwrap_or(default_dim).clamp(0.0, 1.0),
        kind: mask_kind_id(e.kind),
        alpha,
    })
}

fn project_rect(
    rect: [f32; 4],
    scene: &Scene,
    cam: Camera,
    src_full: RectF,
    ow: u32,
    oh: u32,
) -> Option<([f32; 2], [f32; 2])> {
    let (mut mn, mut mx) = ([f32::MAX; 2], [f32::MIN; 2]);
    for (fx, fy) in [
        (rect[0], rect[1]),
        (rect[0] + rect[2], rect[1]),
        (rect[0], rect[1] + rect[3]),
        (rect[0] + rect[2], rect[1] + rect[3]),
    ] {
        let p = FramePoint {
            x: (src_full.x + fx * src_full.w).round() as i32,
            y: (src_full.y + fy * src_full.h).round() as i32,
        };
        let b = to_panel(p, scene.src, scene.screen.rect);
        let (x, y) = project(b.x as f32, b.y as f32, cam, ow, oh);
        mn = [mn[0].min(x), mn[1].min(y)];
        mx = [mx[0].max(x), mx[1].max(y)];
    }
    let r = scene.screen.rect;
    let lo = project(r.x, r.y, cam, ow, oh);
    let hi = project(r.x + r.w, r.y + r.h, cam, ow, oh);
    mn = [mn[0].max(lo.0), mn[1].max(lo.1)];
    mx = [mx[0].min(hi.0), mx[1].min(hi.1)];
    (mx[0] - mn[0] >= 1.0 && mx[1] - mn[1] >= 1.0).then_some((mn, mx))
}

#[cfg(test)]
#[path = "fx_masks_tests.rs"]
mod tests;
