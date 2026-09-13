// Per-type cursor sprite set for Enhanced export: decode each shape once, look up
// the active type per frame, and invert RGB for dark themes (one asset set serves both).
use std::collections::{HashMap, VecDeque};
use crate::events::track::cursortype::{CursorType, CursorTrack};
use crate::events::model::{EventKind, MouseEvent};
use crate::settings::cursor::{CursorSettings, CursorStyle};
use crate::export::cursor::busy::{busy_pose, BusyPose, BusySpec};
use crate::export::cursor::cursordraw::{decode_sprite, CursorSprite};
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint, RectF};

// (CursorType, sprite PNG, hotspot as canvas-fraction). Adding a cursor = add a row.
pub(crate) const SPRITES: &[(CursorType, &[u8], (f32, f32))] = &[
    (CursorType::Arrow,       include_bytes!("../../../assets/cursors/pointer.png"),       (0.155, 0.045)),
    (CursorType::Hand,        include_bytes!("../../../assets/cursors/hand.png"),          (0.49, 0.22)),
    (CursorType::IBeam,       include_bytes!("../../../assets/cursors/ibeam.png"),         (0.50, 0.50)),
    (CursorType::ResizeNs,    include_bytes!("../../../assets/cursors/resize_ns.png"),     (0.50, 0.50)),
    (CursorType::ResizeEw,    include_bytes!("../../../assets/cursors/resize_ew.png"),     (0.50, 0.50)),
    (CursorType::ResizeNwse,  include_bytes!("../../../assets/cursors/resize_nwse.png"),   (0.50, 0.50)),
    (CursorType::ResizeNesw,  include_bytes!("../../../assets/cursors/resize_nesw.png"),   (0.50, 0.50)),
    (CursorType::Move,        include_bytes!("../../../assets/cursors/move.png"),          (0.50, 0.50)),
    (CursorType::Busy,        include_bytes!("../../../assets/cursors/busy.png"),          (0.50, 0.50)),
];

pub struct CursorPrep {
    pub set: HashMap<CursorType, CursorSprite>,
    pub track: CursorTrack,
    pub click_ms: Vec<u32>,
    pub recent: VecDeque<(f32, f32)>,
    /// The selected pack's busy animation (pack format v2), `None` for a still one.
    pub busy: Option<BusySpec>,
    /// Decoded `busy_NN.png` frames when the pack ships them; empty otherwise. Indexed by
    /// `BusyPose::frame`, so an out-of-range index simply falls back to `set`'s busy sprite.
    pub busy_frames: Vec<CursorSprite>,
}

/// Invert RGB in place (black<->white) for a dark-theme cursor; alpha untouched.
fn invert_rgb(bgra: &mut [u8]) {
    for px in bgra.chunks_exact_mut(4) { px[0] = 255 - px[0]; px[1] = 255 - px[1]; px[2] = 255 - px[2]; }
}

/// Whether a synthetic cursor is drawn at all: Enhanced always, System only as the plain-OS
/// stand-in for a video with no baked cursor. Hidden never. Split out of `prep` so the gate is
/// testable without decoding sprites (which shells out to ffmpeg).
pub fn draws_synthetic(cursor: &CursorSettings, os_cursor_in_video: bool) -> bool {
    cursor.style == CursorStyle::Enhanced || cursor.plain_os(os_cursor_in_video)
}

/// Decode every cursor sprite (inverting for dark theme). None unless Enhanced (or plain-OS, see
/// `CursorSettings::plain_os`) or if the Arrow fallback fails to decode. Non-arrow sprites that
/// fail to decode are skipped. Sprite bytes come from `cursor.pack` (the built-in set, or an
/// imported pack falling back to the built-in per-kind) via `pack::sprite_sources`.
pub fn prep(cursor: &CursorSettings, events: &[MouseEvent], track: CursorTrack, dark: bool,
            os_cursor_in_video: bool) -> Option<CursorPrep> {
    if !draws_synthetic(cursor, os_cursor_in_video) { return None; }
    let dark = dark && crate::export::cursor::pack::theme_inverts(&cursor.pack); // coloured packs keep their colours
    let mut set = HashMap::new();
    for (ty, png, hot) in crate::export::cursor::pack::sprite_sources(&cursor.pack) {
        if let Some(mut spr) = decode_sprite(&png, hot) {
            if dark { invert_rgb(&mut spr.bgra); }
            set.insert(ty, spr);
        }
    }
    set.get(&CursorType::Arrow)?; // arrow is the universal fallback - required
    let click_ms = events.iter().filter(|e| e.kind == EventKind::Down).map(|e| e.t).collect();
    // Decoded here, not per frame: an explicit-frame pack is a handful of extra PNG decodes at
    // renderer-build time and zero work afterwards.
    let busy_frames = crate::export::cursor::pack::busy_frames(&cursor.pack).iter()
        .filter_map(|png| decode_sprite(png, (0.5, 0.5)))
        .map(|mut spr| { if dark { invert_rgb(&mut spr.bgra); } spr })
        .collect();
    let busy = crate::export::cursor::pack::busy_spec(&cursor.pack);
    Some(CursorPrep { set, track, click_ms, recent: VecDeque::new(), busy, busy_frames })
}

/// The sprite and transform for this frame: normally the type track's own sprite, still. For the
/// Busy type on a v2 pack it is `busy_pose`'s answer at OUTPUT time `out_t` - an explicit frame
/// when the pack ships them, otherwise the single busy sprite plus a rotation or scale.
///
/// *Why output time and not `ev_t`:* the animation belongs to the rendered timeline, so one
/// instant always yields one pose - deterministic per exported frame, and a paused preview shows
/// exactly the frame for where the playhead sits rather than something that depends on how the
/// user got there.
/// Takes the prep's fields SEPARATELY (like `sprite_for`, and for the same reason): the caller
/// still needs `&mut cp.recent` for the motion trail while holding this sprite, which only works
/// as disjoint field borrows.
fn posed<'a>(set: &'a HashMap<CursorType, CursorSprite>, track: &CursorTrack,
             busy: Option<BusySpec>, frames: &'a [CursorSprite], ev_t: u32, out_t: u32)
             -> (Option<&'a CursorSprite>, BusyPose) {
    let spr = sprite_for(set, track, ev_t);
    if track.type_at(ev_t) != CursorType::Busy { return (spr, BusyPose::still()); }
    let Some(spec) = busy else { return (spr, BusyPose::still()) };
    let pose = busy_pose(&spec, out_t);
    match frames.get(pose.frame as usize) {
        Some(frame) => (Some(frame), BusyPose::still()),
        None => (spr, pose),
    }
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
#[allow(clippy::too_many_arguments)]
pub fn draw(cp: &mut CursorPrep, out: &mut [u8], ow: u32, oh: u32, cur: FramePoint,
            cam: Camera, screen: &Panel, inset_w: f32, ev_t: u32, out_t: u32, c: &CursorSettings,
            os_cursor_in_video: bool) {
    let Some((pos, panel, clip)) = frame_placement(cur, cam, ow, oh, screen, inset_w) else { return };
    // Plain-OS: the recorded type track may not even exist (a Hidden recording has none), and
    // "System" promises a plain arrow rather than Enhanced-minus-polish - so always Arrow, and
    // none of the fake-polish passes a real OS cursor doesn't have. Decided per frame from the
    // LIVE doc settings, not cached on the prep, so switching style in the editor takes effect
    // in the warm preview immediately (the prep itself is only rebuilt on a full renderer build).
    let plain_os = c.plain_os(os_cursor_in_video);
    let (spr, pose) = if plain_os {
        (cp.set.get(&CursorType::Arrow), BusyPose::still())
    } else {
        posed(&cp.set, &cp.track, cp.busy, &cp.busy_frames, ev_t, out_t)
    };
    if let Some(spr) = spr {
        let (blur, bounce) = if plain_os { (0.0, false) } else { (c.motion_blur, c.click_bounce) };
        crate::export::cursor::cursordraw::apply_enhanced(out, ow, oh, spr, pos, &mut cp.recent, 6,
            &cp.click_ms, ev_t, c.size, blur, bounce, c.bounce_intensity, panel, clip, pose);
    }
}

/// Where a cursor goes this frame, whichever cursor it is: the hotspot's on-screen point (`cur`
/// projected through the camera), the panel scale factor (the screen panel's width against the
/// export's fixed `inset_w` reference, so a shrunk custom arrangement shrinks the cursor with
/// it), and the panel's projected clip box. `None` once the screen panel is more than half faded
/// - no screen, no cursor. Shared by the synthetic draw below and the captured-layer draw in
/// `captured.rs`, so the two can never drift apart.
pub fn frame_placement(cur: FramePoint, cam: Camera, ow: u32, oh: u32, screen: &Panel,
                       inset_w: f32) -> Option<((f32, f32), f32, (i32, i32, i32, i32))> {
    if screen.alpha < 0.5 { return None; }
    let pos = crate::export::coordmap::project(cur.x as f32, cur.y as f32, cam, ow, oh);
    let panel = (screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0);
    Some((pos, panel, project_rect(screen.rect, cam, ow, oh)))
}

/// The screen panel rect projected through the camera into final output pixels, clamped to the
/// frame, as an (x0, y0, x1, y1) clip box for the cursor blit.
fn project_rect(r: RectF, cam: Camera, ow: u32, oh: u32) -> (i32, i32, i32, i32) {
    let (x0, y0) = crate::export::coordmap::project(r.x, r.y, cam, ow, oh);
    let (x1, y1) = crate::export::coordmap::project(r.x + r.w, r.y + r.h, cam, ow, oh);
    (x0.max(0.0) as i32, y0.max(0.0) as i32, x1.min(ow as f32) as i32, y1.min(oh as f32) as i32)
}

#[cfg(test)]
#[path = "cursorset_tests.rs"]
mod tests;
