use super::*;
fn tiny_sprite() -> CursorSprite {
    CursorSprite {
        bgra: vec![0, 0, 0, 255],
        w: 1,
        h: 1,
        hot: (0.0, 0.0),
        canvas_h: 1,
    }
}
fn cursor_with(style: CursorStyle) -> CursorSettings {
    let mut c = CursorSettings::default();
    c.style = style;
    c
}

#[test]
fn prep_is_none_for_system_and_hidden_when_the_video_has_the_os_cursor() {
    assert!(prep(
        &cursor_with(CursorStyle::System),
        &[],
        CursorTrack::default(),
        false,
        true
    )
    .is_none());
    assert!(prep(
        &cursor_with(CursorStyle::Hidden),
        &[],
        CursorTrack::default(),
        false,
        true
    )
    .is_none());
    assert!(prep(
        &cursor_with(CursorStyle::Hidden),
        &[],
        CursorTrack::default(),
        false,
        false
    )
    .is_none());
}

#[test]
fn draws_synthetic_covers_every_style_and_bake_combination() {
    let cases = [
        (CursorStyle::System, true, false),
        (CursorStyle::System, false, true),
        (CursorStyle::Enhanced, true, true),
        (CursorStyle::Enhanced, false, true),
        (CursorStyle::Hidden, true, false),
        (CursorStyle::Hidden, false, false),
    ];
    for (style, baked, want) in cases {
        assert_eq!(
            draws_synthetic(&cursor_with(style), baked),
            want,
            "{style:?} baked={baked}"
        );
    }
}

#[test]
fn plain_os_mode_strips_the_polish_and_keeps_the_raw_path() {
    let (sys, enh) = (
        cursor_with(CursorStyle::System),
        cursor_with(CursorStyle::Enhanced),
    );
    assert!(
        sys.plain_os(false) && !sys.plain_os(true),
        "only System + no baked cursor is plain-OS"
    );
    assert!(
        !enh.plain_os(false) && !enh.plain_os(true),
        "Enhanced is never plain-OS"
    );
    assert_eq!(sys.smoothness_at(false), 0.0);
    assert_eq!(sys.idealize_at(false), 0.0);
    assert_eq!(sys.smoothness_at(true), sys.smoothness);
    assert_eq!(enh.smoothness_at(false), enh.smoothness);
}

#[test]
fn invert_rgb_black_to_white_keeps_alpha() {
    let mut px = [0u8, 0, 0, 255];
    invert_rgb(&mut px);
    assert_eq!(px, [255, 255, 255, 255]);
}

fn block(w: u32, h: u32) -> CursorSprite {
    CursorSprite {
        bgra: vec![255; (w * h * 4) as usize],
        w,
        h,
        hot: (0.5, 0.5),
        canvas_h: h,
    }
}

fn prep_of(spr: CursorSprite) -> CursorPrep {
    let mut set = HashMap::new();
    set.insert(CursorType::Arrow, spr);
    CursorPrep {
        set,
        track: CursorTrack::default(),
        click_ms: vec![],
        recent: VecDeque::new(),
        busy: None,
        busy_frames: vec![],
        glass: false,
        masks: HashMap::new(),
        drags: vec![],
    }
}

fn ink_box(out: &[u8], ow: u32) -> (i32, i32, i32, i32) {
    let (mut lo, mut hi) = ((i32::MAX, i32::MAX), (i32::MIN, i32::MIN));
    for (i, px) in out.chunks_exact(4).enumerate() {
        if px[3] == 0 {
            continue;
        }
        let (x, y) = ((i as u32 % ow) as i32, (i as u32 / ow) as i32);
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x + 1), hi.1.max(y + 1));
    }
    (lo.0, lo.1, hi.0, hi.1)
}

#[test]
fn a_tilted_cursor_is_the_same_sprite_rotated_about_its_hotspot() {
    let (ow, oh) = (1000u32, 1000u32);
    let screen = Panel {
        rect: RectF {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 1000.0,
        },
        radius: 0.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    };
    let cam = Camera {
        cx: 500.0,
        cy: 500.0,
        scale: 1.0,
    };
    let c = CursorSettings {
        style: CursorStyle::Enhanced,
        size: 3.0,
        motion_blur: 0.0,
        click_bounce: false,
        ..Default::default()
    };
    let boxed = |tilt: f32| {
        let mut cp = prep_of(block(8, 16));
        let mut out = vec![0u8; (ow * oh * 4) as usize];
        draw(
            &mut cp,
            &mut out,
            ow,
            oh,
            FramePoint { x: 500, y: 500 },
            cam,
            &screen,
            1000.0,
            0,
            0,
            &c,
            true,
            tilt,
        );
        ink_box(&out, ow)
    };
    let (tw, th) = (99.0f32 * 8.0 / 16.0, 99.0f32);
    let flat = boxed(0.0);
    assert_eq!(
        (flat.2 - flat.0, flat.3 - flat.1),
        (tw.round() as i32, th as i32),
        "upright box"
    );
    let lean = boxed(6.0);
    assert_ne!(lean, flat, "6 degrees must actually move pixels");
    let (sin, cos) = (6.0f32.to_radians().sin(), 6.0f32.to_radians().cos());
    let (want_w, want_h) = (tw * cos + th * sin, tw * sin + th * cos);
    assert!(
        ((lean.2 - lean.0) as f32 - want_w).abs() <= 2.0,
        "width {} want {want_w}",
        lean.2 - lean.0
    );
    assert!(
        ((lean.3 - lean.1) as f32 - want_h).abs() <= 2.0,
        "height {} want {want_h}",
        lean.3 - lean.1
    );
    for b in [flat, lean] {
        assert!(
            (((b.0 + b.2) as f32 / 2.0) - 500.0).abs() <= 1.0,
            "centre x of {b:?}"
        );
        assert!(
            (((b.1 + b.3) as f32 / 2.0) - 500.0).abs() <= 1.0,
            "centre y of {b:?}"
        );
    }
}

#[test]
fn sprite_for_empty_track_falls_back_to_arrow() {
    let mut set = HashMap::new();
    set.insert(CursorType::Arrow, tiny_sprite());
    let track = CursorTrack::default();
    assert!(sprite_for(&set, &track, 0).is_some());
    assert!(sprite_for(&set, &track, 9999).is_some());
}
