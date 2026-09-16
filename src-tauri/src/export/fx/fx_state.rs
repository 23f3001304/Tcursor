use crate::actions::model::{ActionEvent, ActionKind};
use crate::edit::model::EffectRegion;
use crate::events::model::{MouseEvent, ScreenInfo};
use crate::export::coordmap::{project, to_frame, to_panel};
use crate::export::fx::click::clickfx::hits_at;
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint};
use crate::settings::model::{
    ClickFxSettings, ClickFxStyle, HotkeySettings, SpotlightMode, VideoFxMode,
};

pub use crate::export::fx::spot::spotlight_sim::SpotlightSim;

const LIFE_MS: u32 = 600;
const FADE_MS: u32 = 250;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxHit {
    pub x: f32,
    pub y: f32,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub cx: f32,
    pub cy: f32,
    pub dim: f32,
    pub radius_frac: f32,
    pub feather_frac: f32,
    pub alpha: f32,
    pub mode: SpotlightMode,
    pub tint: [u8; 3],
    pub t: f32,
    pub cam_rect: [f32; 4],
    pub cam_radius: f32,
    pub dim_camera: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoFx {
    pub mode: VideoFxMode,
    pub alpha: f32,
    pub t: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FxState {
    pub style: ClickFxStyle,
    pub color: [u8; 3],
    pub intensity: f32,
    pub hits: Vec<FxHit>,
    pub masks: Vec<crate::export::fx::fx_masks::MaskDraw>,
    pub spot: Option<Spot>,
    pub video: Option<VideoFx>,
    pub lens: Option<crate::export::fx::lens::Lenses>,
    pub grade: Option<crate::export::grade::GradeParams>,
}

impl Default for FxState {
    fn default() -> Self {
        Self {
            style: ClickFxStyle::None,
            color: [0, 0, 0],
            intensity: 1.0,
            hits: Vec::new(),
            masks: Vec::new(),
            spot: None,
            video: None,
            lens: None,
            grade: None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fx_state_at(
    fx: &ClickFxSettings,
    events: &[MouseEvent],
    actions: &[ActionEvent],
    effects: &[EffectRegion],
    scene: &Scene,
    cam: Camera,
    cur: FramePoint,
    screen: &ScreenInfo,
    has_webcam: bool,
    ow: u32,
    oh: u32,
    region_t: u32,
    ev_t: u32,
    spot_sim: &mut SpotlightSim,
) -> Option<FxState> {
    let s_alpha = spot_sim.resolve(effects, region_t, fx.spotlight);
    let spot = if s_alpha > 0.0 {
        let (cx, cy) = project(cur.x as f32, cur.y as f32, cam, ow, oh);
        let (mode, dim, radius, feather) = spot_sim.style(effects, fx);
        let sfrac = (scene.screen.rect.h / oh.max(1) as f32).max(0.0);
        let cr = scene.camera.rect;
        let has_hole = has_webcam && scene.camera.alpha > 0.05;
        Some(Spot {
            cx,
            cy,
            dim,
            radius_frac: radius * sfrac,
            feather_frac: feather * sfrac,
            alpha: s_alpha,
            mode,
            tint: fx.spotlight_tint,
            t: region_t as f32 / 1000.0,
            cam_rect: if has_hole {
                [cr.x, cr.y, cr.x + cr.w, cr.y + cr.h]
            } else {
                [0.0; 4]
            },
            cam_radius: if has_hole { scene.camera.radius } else { 0.0 },
            dim_camera: if has_hole {
                fx.spotlight_dim_camera
            } else {
                true
            },
        })
    } else {
        None
    };

    let mut hits = Vec::new();
    if !matches!(fx.style, ClickFxStyle::None) {
        for h in hits_at(events, ev_t, LIFE_MS) {
            let p = to_frame(screen, h.sx, h.sy);
            let b = to_panel(p, scene.src, scene.screen.rect);
            let (x, y) = project(b.x as f32, b.y as f32, cam, ow, oh);
            hits.push(FxHit {
                x,
                y,
                progress: h.progress,
            });
        }
    }

    let va = crate::export::fx::hold::hold_alpha(
        actions,
        ev_t,
        FADE_MS,
        |k| matches!(k, ActionKind::VideoFxHoldStart),
        |k| matches!(k, ActionKind::VideoFxHoldEnd),
    );
    let video = if va > 0.0 {
        Some(VideoFx {
            mode: fx.video_fx_mode,
            alpha: va,
            t: ev_t as f32 / 1000.0,
        })
    } else {
        None
    };

    if spot.is_none() && hits.is_empty() && video.is_none() {
        return None;
    }
    Some(FxState {
        style: fx.style,
        color: fx.color,
        intensity: fx.intensity,
        hits,
        spot,
        video,
        ..Default::default()
    })
}

pub trait FxRenderer: Send {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState);
}

pub fn select_fx(ow: u32, oh: u32) -> Box<dyn FxRenderer> {
    if crate::export::gpu::gpu_available() {
        if let Some(g) = crate::export::fx::fx_gpu::GpuFx::new(ow, oh) {
            return Box::new(g);
        }
    }
    Box::new(crate::export::fx::fxdraw::CpuFx)
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    r: &dyn FxRenderer,
    out: &mut [u8],
    ow: u32,
    oh: u32,
    fx: &ClickFxSettings,
    events: &[MouseEvent],
    actions: &[ActionEvent],
    effects: &[EffectRegion],
    scene: &Scene,
    cam: Camera,
    cur: FramePoint,
    screen: &ScreenInfo,
    has_webcam: bool,
    region_t: u32,
    ev_t: u32,
    keys: &HotkeySettings,
    spot_sim: &mut SpotlightSim,
    lens: Option<crate::export::fx::lens::Lenses>,
    masks: Vec<crate::export::fx::fx_masks::MaskDraw>,
    grade: Option<crate::export::grade::GradeParams>,
) {
    let built = fx
        .enabled
        .then(|| {
            fx_state_at(
                fx, events, actions, effects, scene, cam, cur, screen, has_webcam, ow, oh,
                region_t, ev_t, spot_sim,
            )
        })
        .flatten();
    let spill = masks
        .len()
        .saturating_sub(crate::export::fx::fx_uniforms::MAX_MASKS);
    let (keep, over) = masks.split_at(masks.len() - spill);
    if !over.is_empty() {
        crate::export::fx::maskdraw::draw_masks(out, ow, oh, over);
    }
    let masks = keep.to_vec();
    let extra = lens.is_some() || !masks.is_empty() || grade.is_some();
    let mut state = built.or_else(|| {
        extra.then(|| FxState {
            color: fx.color,
            intensity: fx.intensity,
            ..Default::default()
        })
    });
    if let Some(s) = state.as_mut() {
        s.lens = lens;
        s.masks = masks;
        s.grade = grade;
    }
    if let Some(state) = state {
        r.apply(out, ow, oh, &state);
    }
    if !fx.enabled {
        return;
    }
    crate::export::fx::click::hotkeycap::overlay(out, ow, oh, actions, keys, ev_t, fx.captions);
}

#[cfg(test)]
#[path = "fx_state_tests.rs"]
mod tests;
