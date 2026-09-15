use super::overlay;
use crate::edit::captions::{Caption, CaptionWord};
use crate::settings::captions::{CaptionAnim, CaptionPos, CaptionStyle};

fn cap() -> Caption {
    Caption {
        id: "c0".into(),
        start_ms: 0,
        end_ms: 2000,
        text: "hello world".into(),
        words: vec![
            CaptionWord {
                start_ms: 0,
                end_ms: 1000,
                text: "hello".into(),
            },
            CaptionWord {
                start_ms: 1000,
                end_ms: 2000,
                text: "world".into(),
            },
        ],
    }
}

#[test]
fn a_caption_paints_in_the_bottom_band_and_leaves_the_top_alone() {
    let (w, h) = (640u32, 360u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    overlay(
        &mut out,
        w,
        h,
        &[cap()],
        &CaptionStyle::default(),
        [239, 68, 68],
        1000,
    );
    let row = |y: u32| {
        out[((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize]
            .iter()
            .any(|&b| b > 0)
    };
    assert!(row(h - 40), "the bottom band must be painted");
    assert!(!row(10), "the top must be untouched");
}

#[test]
fn top_position_moves_the_paint_to_the_top_band() {
    let (w, h) = (640u32, 360u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    let s = CaptionStyle {
        position: CaptionPos::Top,
        ..Default::default()
    };
    overlay(&mut out, w, h, &[cap()], &s, [239, 68, 68], 1000);
    let row = |y: u32| {
        out[((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize]
            .iter()
            .any(|&b| b > 0)
    };
    assert!(row(40));
    assert!(!row(h - 10));
}

#[test]
fn nothing_is_drawn_when_disabled_between_captions_or_with_an_empty_track() {
    let (w, h) = (320u32, 180u32);
    let blank = vec![7u8; (w * h * 4) as usize];
    let off = CaptionStyle {
        enabled: false,
        ..Default::default()
    };
    for (caps, style, t) in [
        (vec![cap()], off, 1000u32),
        (vec![cap()], CaptionStyle::default(), 5000),
        (vec![], CaptionStyle::default(), 1000),
    ] {
        let mut out = blank.clone();
        overlay(&mut out, w, h, &caps, &style, [239, 68, 68], t);
        assert!(out == blank, "expected a no-op");
    }
}

#[test]
fn turning_the_pill_off_paints_strictly_fewer_pixels_than_leaving_it_on() {
    let (w, h) = (640u32, 360u32);
    let lit = |pill: bool| {
        let mut out = vec![0u8; (w * h * 4) as usize];
        let s = CaptionStyle {
            pill,
            ..Default::default()
        };
        overlay(&mut out, w, h, &[cap()], &s, [239, 68, 68], 1000);
        out.iter().filter(|&&b| b > 0).count()
    };
    assert!(lit(false) < lit(true));
}

#[test]
fn the_highlighted_word_carries_the_accent() {
    let (w, h) = (640u32, 360u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    overlay(
        &mut out,
        w,
        h,
        &[cap()],
        &CaptionStyle::default(),
        [0, 0, 255],
        1500,
    );
    assert!(
        out.chunks(4).any(|p| p[0] > 120 && p[2] < 80),
        "the lit word should be accent-blue"
    );
}

#[test]
fn the_style_can_override_the_text_and_highlight_colours_and_then_the_accent_is_unused() {
    let (w, h) = (640u32, 360u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    let s = CaptionStyle {
        text_color: [0, 255, 0],
        highlight_color: Some([255, 0, 0]),
        ..Default::default()
    };
    overlay(&mut out, w, h, &[cap()], &s, [0, 0, 255], 1500);
    let any = |f: fn(&[u8]) -> bool| out.chunks(4).any(f);
    assert!(any(|p| p[1] > 120 && p[0] < 80 && p[2] < 80), "green body");
    assert!(any(|p| p[2] > 120 && p[0] < 80), "red highlight");
    assert!(!any(|p| p[0] > 120 && p[2] < 80), "the accent is not used");
}

#[test]
fn the_pill_takes_its_own_colour_and_its_own_alpha() {
    let (w, h) = (640u32, 360u32);
    let bright = |pill_alpha: u8| {
        let mut out = vec![0u8; (w * h * 4) as usize];
        let s = CaptionStyle {
            pill_color: [255, 255, 255],
            pill_alpha,
            text_color: [0, 0, 0],
            highlight: false,
            ..Default::default()
        };
        overlay(&mut out, w, h, &[cap()], &s, [239, 68, 68], 1000);
        out.chunks(4)
            .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
            .count()
    };
    assert!(bright(100) > 0, "an opaque white pill must show white");
    assert_eq!(bright(0), 0, "a transparent pill paints nothing");
}

#[test]
fn the_words_animation_reveals_one_word_at_a_time_inside_a_pill_that_never_moves() {
    let (w, h) = (640u32, 360u32);
    let s = CaptionStyle {
        animation: CaptionAnim::Words,
        ..Default::default()
    };
    let shot = |t: u32| {
        let mut out = vec![0u8; (w * h * 4) as usize];
        overlay(&mut out, w, h, &[cap()], &s, [239, 68, 68], t);
        let ink = out
            .chunks(4)
            .filter(|p| p[..3].iter().any(|&b| b > 0))
            .count();
        let pill = out.chunks(4).filter(|p| p[3] > 0).count();
        (ink, pill)
    };
    let (ink_one, pill_one) = shot(500);
    let (ink_two, pill_two) = shot(1500);
    assert!(ink_one > 0 && ink_one < ink_two, "the second word adds ink");
    assert_eq!(
        pill_one, pill_two,
        "the pill is the whole caption's from t0"
    );
}

#[test]
fn the_none_animation_is_already_at_full_strength_on_the_caption_s_first_frame() {
    let (w, h) = (640u32, 360u32);
    let ink = |animation: CaptionAnim| {
        let mut out = vec![0u8; (w * h * 4) as usize];
        let s = CaptionStyle {
            animation,
            ..Default::default()
        };
        overlay(&mut out, w, h, &[cap()], &s, [239, 68, 68], 0);
        out.chunks(4)
            .filter(|p| p[..3].iter().any(|&b| b > 0))
            .count()
    };
    assert_eq!(ink(CaptionAnim::Fade), 0, "the fade starts from nothing");
    assert!(ink(CaptionAnim::None) > 0, "none cuts straight in");
}
