use crate::events::model::{EventKind, MouseEvent};
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::export::cursor::draw::busy::{busy_pose, BusyPose, BusySpec};
use crate::export::cursor::draw::cursordraw::{decode_sprite, CursorSprite};
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint, RectF};
use crate::settings::cursor::{CursorSettings, CursorStyle};
use std::collections::{HashMap, VecDeque};

pub(crate) const SPRITES: &[(CursorType, &[u8], (f32, f32))] = &[
    (
        CursorType::Arrow,
        include_bytes!("../../../../assets/cursors/pointer.png"),
        (0.155, 0.045),
    ),
    (
        CursorType::Hand,
        include_bytes!("../../../../assets/cursors/hand.png"),
        (0.49, 0.22),
    ),
    (
        CursorType::IBeam,
        include_bytes!("../../../../assets/cursors/ibeam.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::ResizeNs,
        include_bytes!("../../../../assets/cursors/resize_ns.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::ResizeEw,
        include_bytes!("../../../../assets/cursors/resize_ew.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::ResizeNwse,
        include_bytes!("../../../../assets/cursors/resize_nwse.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::ResizeNesw,
        include_bytes!("../../../../assets/cursors/resize_nesw.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::Move,
        include_bytes!("../../../../assets/cursors/move.png"),
        (0.50, 0.50),
    ),
    (
        CursorType::Busy,
        include_bytes!("../../../../assets/cursors/busy.png"),
        (0.50, 0.50),
    ),
];

pub struct CursorPrep {
    pub set: HashMap<CursorType, CursorSprite>,
    pub track: CursorTrack,
    pub click_ms: Vec<u32>,
    pub recent: VecDeque<(f32, f32)>,
    pub busy: Option<BusySpec>,
    pub busy_frames: Vec<CursorSprite>,
    pub glass: bool,
    pub masks: HashMap<CursorType, std::sync::Arc<crate::export::fx::fx_lens::LensMask>>,
    pub drags: Vec<crate::export::fx::fx_lens::DragSpan>,
}

fn invert_rgb(bgra: &mut [u8]) {
    for px in bgra.chunks_exact_mut(4) {
        px[0] = 255 - px[0];
        px[1] = 255 - px[1];
        px[2] = 255 - px[2];
    }
}

pub fn draws_synthetic(cursor: &CursorSettings, os_cursor_in_video: bool) -> bool {
    cursor.style == CursorStyle::Enhanced || cursor.plain_os(os_cursor_in_video)
}

pub fn prep(
    cursor: &CursorSettings,
    events: &[MouseEvent],
    track: CursorTrack,
    dark: bool,
    os_cursor_in_video: bool,
) -> Option<CursorPrep> {
    if !draws_synthetic(cursor, os_cursor_in_video) {
        return None;
    }
    let dark = dark && crate::export::cursor::pack::theme_inverts(&cursor.pack);
    let mut set = HashMap::new();
    for (ty, png, hot) in crate::export::cursor::pack::sprite_sources(&cursor.pack) {
        if let Some(mut spr) = decode_sprite(&png, hot) {
            if dark {
                invert_rgb(&mut spr.bgra);
            }
            set.insert(ty, spr);
        }
    }
    set.get(&CursorType::Arrow)?;
    let click_ms = events
        .iter()
        .filter(|e| e.kind == EventKind::Down)
        .map(|e| e.t)
        .collect();
    let busy_frames = crate::export::cursor::pack::busy_frames(&cursor.pack)
        .iter()
        .filter_map(|png| decode_sprite(png, (0.5, 0.5)))
        .map(|mut spr| {
            if dark {
                invert_rgb(&mut spr.bgra);
            }
            spr
        })
        .collect();
    let busy = crate::export::cursor::pack::busy_spec(&cursor.pack);
    let glass = crate::export::cursor::pack::is_glass(&cursor.pack);
    let masks = if glass {
        set.iter()
            .map(|(k, s)| {
                (
                    *k,
                    std::sync::Arc::new(crate::export::fx::fx_lens::mask_of(&cursor.pack, *k, s)),
                )
            })
            .collect()
    } else {
        HashMap::new()
    };
    let drags = crate::export::fx::fx_lens::drag_spans(events);
    Some(CursorPrep {
        set,
        track,
        click_ms,
        recent: VecDeque::new(),
        busy,
        busy_frames,
        glass,
        masks,
        drags,
    })
}

fn posed<'a>(
    set: &'a HashMap<CursorType, CursorSprite>,
    track: &CursorTrack,
    busy: Option<BusySpec>,
    frames: &'a [CursorSprite],
    ev_t: u32,
    out_t: u32,
) -> (Option<&'a CursorSprite>, BusyPose) {
    let spr = sprite_for(set, track, ev_t);
    if track.type_at(ev_t) != CursorType::Busy {
        return (spr, BusyPose::still());
    }
    let Some(spec) = busy else {
        return (spr, BusyPose::still());
    };
    let pose = busy_pose(&spec, out_t);
    match frames.get(pose.frame as usize) {
        Some(frame) => (Some(frame), BusyPose::still()),
        None => (spr, pose),
    }
}

pub fn sprite_for<'a>(
    set: &'a HashMap<CursorType, CursorSprite>,
    track: &CursorTrack,
    ev_t: u32,
) -> Option<&'a CursorSprite> {
    let t = track.type_at(ev_t);
    set.get(&t).or_else(|| set.get(&CursorType::Arrow))
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    cp: &mut CursorPrep,
    out: &mut [u8],
    ow: u32,
    oh: u32,
    cur: FramePoint,
    cam: Camera,
    screen: &Panel,
    inset_w: f32,
    ev_t: u32,
    out_t: u32,
    c: &CursorSettings,
    os_cursor_in_video: bool,
    tilt_deg: f32,
) {
    let Some((pos, panel, clip)) = frame_placement(cur, cam, ow, oh, screen, inset_w) else {
        return;
    };
    let plain_os = c.plain_os(os_cursor_in_video);
    let tilt = if plain_os { 0.0 } else { tilt_deg };
    if cp.glass && !plain_os {
        let (prev, kind, m, angle) = crate::export::fx::fx_lensbuild::morph_at(cp, ev_t, out_t);
        crate::export::cursor::draw::cursormorph::draw_glass(
            cp,
            out,
            ow,
            oh,
            pos,
            prev,
            kind,
            m,
            angle + tilt,
            panel,
            clip,
            ev_t,
            c,
            crate::export::fx::fx_lens::SPRITE_ALPHA,
        );
        return;
    }
    let (spr, mut pose) = if plain_os {
        (cp.set.get(&CursorType::Arrow), BusyPose::still())
    } else {
        posed(&cp.set, &cp.track, cp.busy, &cp.busy_frames, ev_t, out_t)
    };
    pose.angle_deg += tilt;
    if let Some(spr) = spr {
        let (blur, bounce) = if plain_os {
            (0.0, false)
        } else {
            (c.motion_blur, c.click_bounce)
        };
        crate::export::cursor::draw::cursordraw::apply_enhanced(
            out,
            ow,
            oh,
            spr,
            pos,
            &mut cp.recent,
            6,
            &cp.click_ms,
            ev_t,
            c.size,
            blur,
            bounce,
            c.bounce_intensity,
            panel,
            clip,
            pose,
            1.0,
        );
    }
}

pub fn frame_placement(
    cur: FramePoint,
    cam: Camera,
    ow: u32,
    oh: u32,
    screen: &Panel,
    inset_w: f32,
) -> Option<((f32, f32), f32, (i32, i32, i32, i32))> {
    if screen.alpha < 0.5 {
        return None;
    }
    let pos = crate::export::coordmap::project(cur.x as f32, cur.y as f32, cam, ow, oh);
    let panel = (screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0);
    Some((pos, panel, project_rect(screen.rect, cam, ow, oh)))
}

fn project_rect(r: RectF, cam: Camera, ow: u32, oh: u32) -> (i32, i32, i32, i32) {
    let (x0, y0) = crate::export::coordmap::project(r.x, r.y, cam, ow, oh);
    let (x1, y1) = crate::export::coordmap::project(r.x + r.w, r.y + r.h, cam, ow, oh);
    (
        x0.max(0.0) as i32,
        y0.max(0.0) as i32,
        x1.min(ow as f32) as i32,
        y1.min(oh as f32) as i32,
    )
}

#[cfg(test)]
#[path = "cursorset_tests.rs"]
mod tests;
