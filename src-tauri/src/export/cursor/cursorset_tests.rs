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
fn prep_is_none_for_system_and_hidden_when_the_video_has_the_os_cursor() {
    // Style gate only; no decode happens (which would need ffmpeg).
    assert!(prep(&cursor_with(CursorStyle::System), &[], CursorTrack::default(), false, true).is_none());
    assert!(prep(&cursor_with(CursorStyle::Hidden), &[], CursorTrack::default(), false, true).is_none());
    // Hidden means "no cursor" whatever the video holds - never a fallback.
    assert!(prep(&cursor_with(CursorStyle::Hidden), &[], CursorTrack::default(), false, false).is_none());
}

#[test]
fn draws_synthetic_covers_every_style_and_bake_combination() {
    // The reported bug is the second row: an Enhanced recording (no OS cursor in the pixels)
    // switched to System in the editor drew nothing at all.
    let cases = [
        (CursorStyle::System,   true,  false), // baked cursor already in the video -> draw nothing
        (CursorStyle::System,   false, true),  // THE FIX: no baked cursor -> plain-OS stand-in
        (CursorStyle::Enhanced, true,  true),  // Enhanced is unchanged either way
        (CursorStyle::Enhanced, false, true),
        (CursorStyle::Hidden,   true,  false), // Hidden means no cursor, full stop
        (CursorStyle::Hidden,   false, false),
    ];
    for (style, baked, want) in cases {
        assert_eq!(draws_synthetic(&cursor_with(style), baked), want, "{style:?} baked={baked}");
    }
}

#[test]
fn plain_os_mode_strips_the_polish_and_keeps_the_raw_path() {
    let (sys, enh) = (cursor_with(CursorStyle::System), cursor_with(CursorStyle::Enhanced));
    assert!(sys.plain_os(false) && !sys.plain_os(true), "only System + no baked cursor is plain-OS");
    assert!(!enh.plain_os(false) && !enh.plain_os(true), "Enhanced is never plain-OS");
    // Raw recorded path: alpha 1.0 makes Cursor::at return the interpolated sample verbatim.
    assert_eq!(sys.follow_alpha_at(false), 1.0);
    assert_eq!(sys.idealize_at(false), 0.0);
    // Every other combination keeps the user's smoothing exactly as before.
    assert_eq!(sys.follow_alpha_at(true), sys.follow_alpha());
    assert_eq!(enh.follow_alpha_at(false), enh.follow_alpha());
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
