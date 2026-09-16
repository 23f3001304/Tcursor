use super::*;
use crate::edit::text::{TextAnchor, TextAnim, TextItem, TextKind, TextSize};

const ACCENT: [u8; 3] = [239, 68, 68];

fn item(pos: TextAnchor, style: &str) -> TextItem {
    TextItem {
        id: "t0".into(),
        start_ms: 0,
        end_ms: 4000,
        kind: TextKind::Title,
        text: "Hello".into(),
        sub: None,
        style: style.into(),
        pos,
        offset: [0.0, 0.0],
        size: TextSize::L,
        anim_in: TextAnim::Fade,
        anim_out: TextAnim::Fade,
        in_ms: 1,
        out_ms: 1,
        easing: "linear".into(),
    }
}

fn band(out: &[u8], w: u32, h: u32, row: usize, col: usize) -> u32 {
    let (x0, x1) = (col as u32 * w / 3, (col as u32 + 1) * w / 3);
    let (y0, y1) = (row as u32 * h / 3, (row as u32 + 1) * h / 3);
    let mut lit = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if out[((y * w + x) * 4) as usize] > 30 {
                lit += 1;
            }
        }
    }
    lit
}

#[test]
fn a_title_paints_in_the_band_its_anchor_names_and_leaves_the_others_alone() {
    let (w, h) = (480u32, 300u32);
    let anchors = [
        (TextAnchor::TopLeft, 0, 0),
        (TextAnchor::TopCenter, 0, 1),
        (TextAnchor::TopRight, 0, 2),
        (TextAnchor::MidLeft, 1, 0),
        (TextAnchor::MidCenter, 1, 1),
        (TextAnchor::MidRight, 1, 2),
        (TextAnchor::BottomLeft, 2, 0),
        (TextAnchor::BottomCenter, 2, 1),
        (TextAnchor::BottomRight, 2, 2),
    ];
    for (pos, r, c) in anchors {
        let mut out = vec![0u8; (w * h * 4) as usize];
        overlay(&mut out, w, h, &[item(pos, "clean")], ACCENT, 2000);
        let mine = band(&out, w, h, r, c);
        assert!(
            mine > 20,
            "{pos:?} painted only {mine} pixels in its own band"
        );
        let far_r = if r == 0 { 2 } else { 0 };
        let far_c = if c == 0 { 2 } else { 0 };
        assert_eq!(
            band(&out, w, h, far_r, far_c),
            0,
            "{pos:?} bled into the opposite band"
        );
    }
}

#[test]
fn alpha_zero_is_a_byte_identical_no_op() {
    let (w, h) = (200u32, 120u32);
    let mut out: Vec<u8> = (0..(w * h * 4)).map(|i| (i % 251) as u8).collect();
    let before = out.clone();
    let mut it = item(TextAnchor::MidCenter, "plate");
    it.end_ms = 1;
    overlay(&mut out, w, h, &[it], ACCENT, 5000);
    assert_eq!(out, before, "a text that is not live must not touch a byte");
}

#[test]
fn the_plate_style_paints_a_scrim_and_the_bar_style_paints_a_rule_in_the_accent() {
    let (w, h) = (400u32, 240u32);
    let mut plate = vec![255u8; (w * h * 4) as usize];
    overlay(
        &mut plate,
        w,
        h,
        &[item(TextAnchor::MidCenter, "plate")],
        ACCENT,
        2000,
    );
    assert!(
        plate.chunks(4).any(|p| p[0] < 150),
        "the plate darkened the frame behind the run"
    );

    let mut bar = vec![0u8; (w * h * 4) as usize];
    overlay(
        &mut bar,
        w,
        h,
        &[item(TextAnchor::BottomLeft, "bar")],
        ACCENT,
        2000,
    );
    assert!(
        bar.chunks(4).any(|p| p[2] > 200 && p[0] < 120),
        "the rule is the document accent, which is red in the R byte at index 2"
    );
}

#[test]
fn a_typewriter_draws_only_the_revealed_characters() {
    let (w, h) = (600u32, 200u32);
    let mut it = item(TextAnchor::MidLeft, "clean");
    it.text = "ABCDEFGHIJ".into();
    it.anim_in = TextAnim::Typewriter;
    it.in_ms = 1000;
    let lit = |t: u32| {
        let mut out = vec![0u8; (w * h * 4) as usize];
        overlay(&mut out, w, h, &[it.clone()], ACCENT, t);
        out.chunks(4).filter(|p| p[0] > 30).count()
    };
    let early = lit(200);
    let late = lit(900);
    assert!(
        early > 0 && late > early,
        "more glyphs as the reveal advances: {early} {late}"
    );
}

#[test]
fn the_pass_order_is_text_over_the_effects_and_under_the_captions() {
    let src = include_str!("../../render/fx_step.rs");
    let fx = src.find("fx_state::render").expect("the FX pass call");
    let text = src.find("textdraw::overlay").expect("the text call");
    let cap = src.find("captiondraw::overlay").expect("the caption call");
    assert!(
        fx < text && text < cap,
        "fx_pass must run the FX pass, then text, then captions"
    );
}
