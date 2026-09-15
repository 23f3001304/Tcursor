use crate::events::model::ScreenInfo;
use crate::events::track::cursortype::CursorType;
use crate::export::coordmap::{project, to_frame, to_panel};
use crate::export::cursor::busy::busy_pose;
use crate::export::cursor::cursordraw::bounce_scale;
use crate::export::cursor::cursormorph::morph_box;
use crate::export::cursor::cursorset::{frame_placement, CursorPrep};
use crate::export::fx::lens::{
    back_geom, ink_at, kind_morph, morph_mask, selection_at, squash_at, CursorLens, Lenses,
};
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint};
use crate::settings::cursor::{CursorBack, CursorSettings};

#[derive(Clone, Copy)]
pub struct LensFrame<'a> {
    pub cur: FramePoint,
    pub cam: Camera,
    pub ow: u32,
    pub oh: u32,
    pub screen: &'a Panel,
    pub inset_w: f32,
    pub info: &'a ScreenInfo,
    pub src: crate::export::types::RectF,
    pub ev_t: u32,
    pub out_t: u32,
    pub tilt_deg: f32,
}

pub fn lenses_at(
    cp: &CursorPrep,
    c: &CursorSettings,
    f: LensFrame,
    plain_os: bool,
) -> Option<Lenses> {
    let back_on = c.back == CursorBack::Glass && !plain_os;
    if (!cp.glass || plain_os) && !back_on {
        return None;
    }
    let (pos, panel, _) = frame_placement(f.cur, f.cam, f.ow, f.oh, f.screen, f.inset_w)?;
    let size_px = c.size.clamp(0.4, 3.0) * f.oh as f32 * 0.033 * panel;
    let (kind, prev, morph) = if plain_os {
        (CursorType::Arrow, CursorType::Arrow, 1.0)
    } else {
        kind_morph(&cp.track, f.ev_t)
    };
    let bounce = bounce_scale(
        &cp.click_ms,
        f.ev_t,
        c.click_bounce && !plain_os,
        c.bounce_intensity,
    );
    let squash = squash_at(&cp.click_ms, f.ev_t);
    let ink = ink_at(&cp.click_ms, f.ev_t);
    let fallback = cp.set.get(&CursorType::Arrow);
    let spr = cp.set.get(&kind).or(fallback)?;
    let prev_spr = cp.set.get(&prev).or(fallback)?;
    let b = morph_box(prev_spr, spr, morph, pos, size_px, bounce);
    let (tw, th) = (b[2], b[3]);
    let centre = [b[0] + tw * 0.5, b[1] + th * 0.5];

    let glass = (cp.glass && !plain_os)
        .then(|| {
            lens_of(
                cp,
                kind,
                prev,
                morph,
                centre,
                [tw, th],
                f,
                if c.click_bounce { 1.0 } else { squash },
                ink,
                pos,
            )
        })
        .flatten();
    let back = back_on.then(|| {
        let sel = selection_at(&cp.drags, f.ev_t).map(|(a, w)| (desktop_to_out(a, f), w));
        back_geom(kind, prev, morph, centre, th.max(1.0), sel, squash, ink)
    });
    (glass.is_some() || back.is_some()).then_some(Lenses { glass, back })
}

#[allow(clippy::too_many_arguments)]
fn lens_of(
    cp: &CursorPrep,
    kind: CursorType,
    prev: CursorType,
    m: f32,
    centre: [f32; 2],
    size: [f32; 2],
    f: LensFrame,
    squash: f32,
    ink: f32,
    pos: (f32, f32),
) -> Option<CursorLens> {
    let fallback = cp.masks.get(&CursorType::Arrow);
    let cur = cp.masks.get(&kind).or(fallback)?;
    let mask = match cp.masks.get(&prev).or(fallback) {
        Some(p) if m < 1.0 && prev != kind => std::sync::Arc::new(morph_mask(p, cur, m)),
        _ => cur.clone(),
    };
    let (.., angle) = morph_at(cp, f.ev_t, f.out_t);
    let angle = angle + f.tilt_deg;
    Some(CursorLens {
        cbox: [centre[0], centre[1], size[0], size[1]],
        angle: angle.to_radians(),
        squash,
        ink,
        ink_at: [pos.0, pos.1],
        mask,
    })
}

pub fn morph_at(cp: &CursorPrep, ev_t: u32, out_t: u32) -> (CursorType, CursorType, f32, f32) {
    let (kind, prev, m) = kind_morph(&cp.track, ev_t);
    let angle = match cp.busy {
        Some(spec) if cp.busy_frames.is_empty() => {
            let a = busy_pose(&spec, out_t).angle_deg;
            if kind == CursorType::Busy {
                a * m
            } else if prev == CursorType::Busy {
                a * (1.0 - m)
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    (prev, kind, m, angle)
}

fn desktop_to_out(p: [i32; 2], f: LensFrame) -> [f32; 2] {
    let fr = to_frame(f.info, p[0], p[1]);
    let b = to_panel(fr, f.src, f.screen.rect);
    let (x, y) = project(b.x as f32, b.y as f32, f.cam, f.ow, f.oh);
    [x, y]
}

pub fn wants_lens(cp: Option<&CursorPrep>, c: &CursorSettings, plain_os: bool) -> bool {
    if plain_os {
        return false;
    }
    c.back == CursorBack::Glass || cp.is_some_and(|p| p.glass)
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
