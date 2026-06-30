// Per-type cursor sprite set for Enhanced export: decode each shape once, look up
// the active type per frame, and invert RGB for dark themes (one asset set serves both).
use std::collections::{HashMap, VecDeque};
use crate::events::cursortype::{CursorType, CursorTrack};
use crate::events::model::{EventKind, MouseEvent};
use crate::settings::model::{CursorSettings, CursorStyle};
use crate::export::cursordraw::{decode_sprite, CursorSprite};
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint, RectF};

// (CursorType, sprite PNG, hotspot as canvas-fraction). Adding a cursor = add a row.
pub(crate) const SPRITES: &[(CursorType, &[u8], (f32, f32))] = &[
    (CursorType::Arrow,       include_bytes!("../../assets/cursors/pointer.png"),       (0.155, 0.045)),
    (CursorType::Hand,        include_bytes!("../../assets/cursors/hand.png"),          (0.49, 0.22)),
    (CursorType::IBeam,       include_bytes!("../../assets/cursors/ibeam.png"),         (0.50, 0.50)),
    (CursorType::ResizeNs,    include_bytes!("../../assets/cursors/resize_ns.png"),     (0.50, 0.50)),
    (CursorType::ResizeEw,    include_bytes!("../../assets/cursors/resize_ew.png"),     (0.50, 0.50)),
    (CursorType::ResizeNwse,  include_bytes!("../../assets/cursors/resize_nwse.png"),   (0.50, 0.50)),
    (CursorType::ResizeNesw,  include_bytes!("../../assets/cursors/resize_nesw.png"),   (0.50, 0.50)),
    (CursorType::Move,        include_bytes!("../../assets/cursors/move.png"),          (0.50, 0.50)),
    (CursorType::Busy,        include_bytes!("../../assets/cursors/busy.png"),          (0.50, 0.50)),
];

pub struct CursorPrep {
    pub set: HashMap<CursorType, CursorSprite>,
    pub track: CursorTrack,
    pub click_ms: Vec<u32>,
    pub recent: VecDeque<(f32, f32)>,
}

/// Invert RGB in place (black<->white) for a dark-theme cursor; alpha untouched.
fn invert_rgb(bgra: &mut [u8]) {
    for px in bgra.chunks_exact_mut(4) { px[0] = 255 - px[0]; px[1] = 255 - px[1]; px[2] = 255 - px[2]; }
}

/// Decode every cursor sprite (inverting for dark theme). None unless Enhanced or if the
/// Arrow fallback fails to decode. Non-arrow sprites that fail to decode are skipped.
pub fn prep(cursor: &CursorSettings, events: &[MouseEvent], track: CursorTrack, dark: bool) -> Option<CursorPrep> {
    if cursor.style != CursorStyle::Enhanced { return None; }
    let mut set = HashMap::new();
    for &(ty, png, hot) in SPRITES {
        if let Some(mut spr) = decode_sprite(png, hot) {
            if dark { invert_rgb(&mut spr.bgra); }
            set.insert(ty, spr);
        }
    }
    set.get(&CursorType::Arrow)?; // arrow is the universal fallback - required
    let click_ms = events.iter().filter(|e| e.kind == EventKind::Down).map(|e| e.t).collect();
    Some(CursorPrep { set, track, click_ms, recent: VecDeque::new() })
}

/// The sprite for the cursor type active at `ev_t`, falling back to Arrow.
/// Takes `&set` and `&track` (NOT `&CursorPrep`) so the caller can still mutably
/// borrow `prep.recent` for the trail at the same time (disjoint field borrows).
pub fn sprite_for<'a>(set: &'a HashMap<CursorType, CursorSprite>, track: &CursorTrack, ev_t: u32) -> Option<&'a CursorSprite> {
    let t = track.type_at(ev_t);
    set.get(&t).or_else(|| set.get(&CursorType::Arrow))
}

/// Per-frame Enhanced draw, confined to the screen panel. No-op when the screen panel is absent
/// (alpha < 0.5). `cur` is the cursor in base/output coords; it is projected through `cam`,
/// scaled to the panel size (so a small PiP screen gets a small cursor), and clipped to the
/// panel's on-screen rect so it never spills onto the background or the webcam.
pub fn draw(cp: &mut CursorPrep, out: &mut [u8], ow: u32, oh: u32, cur: FramePoint,
            cam: Camera, screen: &Panel, inset_w: f32, ev_t: u32, c: &CursorSettings) {
    if screen.alpha < 0.5 { return; }
    let pos = crate::export::coordmap::project(cur.x as f32, cur.y as f32, cam, ow, oh);
    let panel = (screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0);
    let clip = project_rect(screen.rect, cam, ow, oh);
    if let Some(spr) = sprite_for(&cp.set, &cp.track, ev_t) {
        crate::export::cursordraw::apply_enhanced(out, ow, oh, spr, pos, &mut cp.recent, 6,
            &cp.click_ms, ev_t, c.size, c.motion_blur, c.click_bounce, c.bounce_intensity, panel, clip);
    }
}

/// The screen panel rect projected through the camera into final output pixels, clamped to the
/// frame, as an (x0, y0, x1, y1) clip box for the cursor blit.
fn project_rect(r: RectF, cam: Camera, ow: u32, oh: u32) -> (i32, i32, i32, i32) {
    let (x0, y0) = crate::export::coordmap::project(r.x, r.y, cam, ow, oh);
    let (x1, y1) = crate::export::coordmap::project(r.x + r.w, r.y + r.h, cam, ow, oh);
    (x0.max(0.0) as i32, y0.max(0.0) as i32, x1.min(ow as f32) as i32, y1.min(oh as f32) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_sprite() -> CursorSprite {
        CursorSprite { bgra: vec![0, 0, 0, 255], w: 1, h: 1, hot: (0.0, 0.0), canvas_h: 1 }
    }
    fn cursor_with(style: CursorStyle) -> CursorSettings {
        let mut c = CursorSettings::default();
        c.style = style;
        c
    }

    #[test]
    fn prep_is_none_for_system_and_hidden() {
        // Style gate only; no decode happens (which would need ffmpeg).
        assert!(prep(&cursor_with(CursorStyle::System), &[], CursorTrack::default(), false).is_none());
        assert!(prep(&cursor_with(CursorStyle::Hidden), &[], CursorTrack::default(), false).is_none());
    }

    #[test]
    fn invert_rgb_black_to_white_keeps_alpha() {
        let mut px = [0u8, 0, 0, 255];
        invert_rgb(&mut px);
        assert_eq!(px, [255, 255, 255, 255]);
    }

    #[test]
    fn sprite_for_empty_track_falls_back_to_arrow() {
        let mut set = HashMap::new();
        set.insert(CursorType::Arrow, tiny_sprite());
        let track = CursorTrack::default();
        // type_at -> Arrow on empty track, and Arrow is present.
        assert!(sprite_for(&set, &track, 0).is_some());
        // A type absent from the map also falls back to Arrow.
        assert!(sprite_for(&set, &track, 9999).is_some());
    }
}
