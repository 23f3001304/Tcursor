use super::{caption_at, layout};
use crate::edit::captions::{Caption, CaptionWord};
use crate::settings::captions::{CaptionAnim, CaptionPos, CaptionSize, CaptionStyle};

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 1e-3, "{what}: {a} != {b}");
}

fn hello_world() -> Caption {
    Caption {
        id: "c0".into(),
        start_ms: 1000,
        end_ms: 3000,
        text: "hello world".into(),
        words: vec![
            CaptionWord {
                start_ms: 1000,
                end_ms: 1500,
                text: "hello".into(),
            },
            CaptionWord {
                start_ms: 1500,
                end_ms: 3000,
                text: "world".into(),
            },
        ],
    }
}

#[test]
fn case_a_bottom_medium_one_line_at_1920x1080() {
    let s = CaptionStyle::default();
    let l = layout(&hello_world(), &s, 1920, 1080, 2000);
    assert_eq!(l.lines, vec!["hello world"]);
    near(l.font_px, 41.04, "font_px");
    near(l.line_h, 54.1728, "line_h");
    near(l.pill[0], 818.0016, "pill x");
    near(l.pill[1], 916.92, "pill y");
    near(l.pill[2], 283.9968, "pill w");
    near(l.pill[3], 82.08, "pill h");
    assert_eq!(l.baselines.len(), 1);
    near(l.baselines[0], 971.9136, "baseline 0");
    assert_eq!(l.hi, Some(1), "at 2000 ms the second word is being spoken");
    near(l.alpha, 1.0, "alpha");
}

#[test]
fn case_b_top_small_two_lines_mid_fade_at_1280x720() {
    let s = CaptionStyle {
        position: CaptionPos::Top,
        size: CaptionSize::S,
        highlight: false,
        ..Default::default()
    };
    let c = Caption {
        id: "c0".into(),
        start_ms: 0,
        end_ms: 4000,
        text: "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii".into(),
        words: vec![],
    };
    let l = layout(&c, &s, 1280, 720, 60);
    assert_eq!(
        l.lines,
        vec!["aaaa bbbb cccc dddd eeee ffff gggg hhhh", "iiii"]
    );
    near(l.font_px, 21.6, "font_px");
    near(l.line_h, 28.512, "line_h");
    near(l.pill[0], 408.016, "pill x");
    near(l.pill[1], 54.0, "pill y");
    near(l.pill[2], 463.968, "pill w");
    near(l.pill[3], 71.712, "pill h");
    near(l.baselines[0], 82.944, "baseline 0");
    near(l.baselines[1], 111.456, "baseline 1");
    assert_eq!(l.hi, None, "highlight off, and no words anyway");
    near(l.alpha, 0.5, "60 of a 120 ms fade");
}

#[test]
fn the_caption_fades_out_symmetrically_and_is_gone_outside_its_span() {
    let s = CaptionStyle::default();
    let c = hello_world();
    near(
        layout(&c, &s, 1920, 1080, 2940).alpha,
        0.5,
        "60 ms left of a 120 ms fade",
    );
    near(
        layout(&c, &s, 1920, 1080, 1000).alpha,
        0.0,
        "exactly at the start",
    );
    near(
        layout(&c, &s, 1920, 1080, 3000).alpha,
        0.0,
        "exactly at the end",
    );
}

#[test]
fn caption_at_picks_the_one_covering_the_instant_and_nothing_at_a_gap() {
    let caps = vec![
        hello_world(),
        Caption {
            id: "c1".into(),
            start_ms: 4000,
            end_ms: 5000,
            text: "next".into(),
            words: vec![],
        },
    ];
    assert_eq!(caption_at(&caps, 2000).map(|c| c.id.as_str()), Some("c0"));
    assert_eq!(caption_at(&caps, 3500).map(|c| c.id.as_str()), None);
    assert_eq!(caption_at(&caps, 4999).map(|c| c.id.as_str()), Some("c1"));
    assert_eq!(
        caption_at(&caps, 3000).map(|c| c.id.as_str()),
        None,
        "end is exclusive"
    );
    assert!(caption_at(&[], 0).is_none());
}

#[test]
fn the_highlight_index_tracks_the_word_being_spoken_and_holds_the_last_one_through_a_pause() {
    let s = CaptionStyle::default();
    let c = hello_world();
    assert_eq!(layout(&c, &s, 1920, 1080, 1100).hi, Some(0));
    assert_eq!(layout(&c, &s, 1920, 1080, 1500).hi, Some(1));
    let late = Caption {
        start_ms: 0,
        ..hello_world()
    };
    assert_eq!(layout(&late, &s, 1920, 1080, 500).hi, None);
}

#[test]
fn an_empty_caption_lays_out_to_nothing_rather_than_a_bare_pill() {
    let s = CaptionStyle::default();
    let c = Caption {
        id: "c0".into(),
        start_ms: 0,
        end_ms: 1000,
        text: "   ".into(),
        words: vec![],
    };
    let l = layout(&c, &s, 1920, 1080, 500);
    assert!(l.lines.is_empty());
    near(l.pill[2], 0.0, "pill w");
    near(l.pill[3], 0.0, "pill h");
}

#[test]
fn wrap_lines_breaks_on_words_and_never_drops_one() {
    use super::wrap_lines;
    assert_eq!(wrap_lines("", 42), Vec::<String>::new());
    assert_eq!(wrap_lines("one two", 42), vec!["one two"]);
    assert_eq!(wrap_lines("aaaa bbbb cc", 9), vec!["aaaa bbbb", "cc"]);
    assert_eq!(
        wrap_lines("supercalifragilistic ok", 8),
        vec!["supercalifragilistic", "ok"]
    );
}

// (anim, t_ms, alpha, rise, scale, words_shown with -1 for none, word_alpha).
// The same 20 rows live in captionPreview.test.ts, so either side drifting is a red test.
// Subject: `hello_world` (1000..3000, words at 1000 and 1500) at 1920x1080, animation_ms 120.
#[rustfmt::skip]
const PARITY: [(CaptionAnim, u32, f32, f32, f32, i32, f32); 20] = [
    (CaptionAnim::None,  1060, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::None,  2000, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::None,  2940, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::None,  3100, 0.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Fade,  1060, 0.5, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Fade,  2000, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Fade,  2940, 0.5, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Fade,  3100, 0.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Rise,  1060, 0.5, 3.3858, 1.0,  -1, 1.0),
    (CaptionAnim::Rise,  2000, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Rise,  2940, 0.5, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Rise,  3100, 0.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Pop,   1060, 0.5, 0.0,    0.99, -1, 1.0),
    (CaptionAnim::Pop,   2000, 1.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Pop,   2940, 0.5, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Pop,   3100, 0.0, 0.0,    1.0,  -1, 1.0),
    (CaptionAnim::Words, 1060, 0.5, 0.0,    1.0,   1, 0.5),
    (CaptionAnim::Words, 2000, 1.0, 0.0,    1.0,   2, 1.0),
    (CaptionAnim::Words, 2940, 0.5, 0.0,    1.0,   2, 1.0),
    (CaptionAnim::Words, 3100, 0.0, 0.0,    1.0,   2, 1.0),
];

#[test]
fn every_animation_kind_is_pinned_at_entry_steady_exit_and_past_the_end() {
    let c = hello_world();
    for (anim, t, alpha, rise, scale, shown, word_alpha) in PARITY {
        let s = CaptionStyle {
            animation: anim,
            ..Default::default()
        };
        let l = layout(&c, &s, 1920, 1080, t);
        let at = format!("{anim:?} at {t}");
        near(l.alpha, alpha, &format!("{at} alpha"));
        near(l.rise, rise, &format!("{at} rise"));
        near(l.scale, scale, &format!("{at} scale"));
        near(l.word_alpha, word_alpha, &format!("{at} word alpha"));
        assert_eq!(
            l.words_shown,
            (shown >= 0).then_some(shown as usize),
            "{at} words shown"
        );
        near(l.font_px, 41.04 * scale, &format!("{at} font px"));
        // The pill's BOTTOM is pinned at the margin, so a pop grows it upward.
        near(
            l.pill[1],
            999.0 - 82.08 * scale + rise,
            &format!("{at} pill y"),
        );
    }
}

#[test]
fn the_rise_offset_moves_the_pill_and_its_baselines_together() {
    let s = CaptionStyle {
        animation: CaptionAnim::Rise,
        ..Default::default()
    };
    let l = layout(&hello_world(), &s, 1920, 1080, 1060);
    near(l.pill[1], 916.92 + 3.3858, "pill y");
    near(l.baselines[0], 971.9136 + 3.3858, "baseline 0");
}

#[test]
fn a_pop_relays_the_pill_out_around_the_scaled_font() {
    let s = CaptionStyle {
        animation: CaptionAnim::Pop,
        ..Default::default()
    };
    let l = layout(&hello_world(), &s, 1920, 1080, 1000);
    near(l.scale, 0.92, "scale at the very start");
    near(l.font_px, 41.04 * 0.92, "font px");
    near(l.pill[2], 283.9968 * 0.92, "pill w shrinks with the text");
    near(l.pill[3], 82.08 * 0.92, "pill h shrinks with the text");
}

#[test]
fn words_falls_back_to_the_fade_when_the_caption_has_no_word_timings() {
    let s = CaptionStyle {
        animation: CaptionAnim::Words,
        ..Default::default()
    };
    let c = Caption {
        words: vec![],
        ..hello_world()
    };
    let l = layout(&c, &s, 1920, 1080, 1060);
    assert_eq!(l.words_shown, None);
    near(l.alpha, 0.5, "the plain fade");
}

#[test]
fn a_zero_length_animation_cuts_in_at_full_alpha_with_no_offset_or_scale() {
    for anim in [
        CaptionAnim::Fade,
        CaptionAnim::Rise,
        CaptionAnim::Pop,
        CaptionAnim::Words,
    ] {
        let s = CaptionStyle {
            animation: anim,
            animation_ms: 0,
            ..Default::default()
        };
        let l = layout(&hello_world(), &s, 1920, 1080, 1000);
        near(l.alpha, 1.0, "alpha");
        near(l.rise, 0.0, "rise");
        near(l.scale, 1.0, "scale");
        near(l.word_alpha, 1.0, "word alpha");
    }
}

#[test]
fn a_fine_font_percent_overrides_the_size_rung_and_is_clamped() {
    let fine = |pct: f32| {
        let s = CaptionStyle {
            font_pct: pct,
            ..Default::default()
        };
        layout(&hello_world(), &s, 1920, 1080, 2000).font_px
    };
    near(fine(0.0), 41.04, "0 keeps the M rung");
    near(fine(5.0), 54.0, "5% of 1080");
    near(fine(0.5), 1080.0 * 0.015, "clamped up to 1.5%");
    near(fine(40.0), 1080.0 * 0.08, "clamped down to 8%");
}

#[test]
fn word_span_walks_the_joined_text_by_character_offset() {
    use super::word_span;
    let c = hello_world();
    assert_eq!(word_span(&c, 0), Some((0, 5)));
    assert_eq!(word_span(&c, 1), Some((6, 11)));
    assert_eq!(word_span(&c, 2), None);
}

#[test]
fn ease_out_is_the_cubic_both_renderers_spell_the_same_way() {
    use super::ease_out;
    near(ease_out(0.0), 0.0, "start");
    near(ease_out(0.5), 0.875, "midpoint");
    near(ease_out(1.0), 1.0, "end");
    near(ease_out(-1.0), 0.0, "clamped below");
    near(ease_out(2.0), 1.0, "clamped above");
}
