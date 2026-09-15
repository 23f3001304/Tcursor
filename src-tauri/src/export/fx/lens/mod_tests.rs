use super::*;
use crate::events::model::{Button, EventKind, MouseEvent};
use crate::export::cursor::cursordraw::CursorSprite;

fn ev(t: u32, kind: EventKind, x: i32, y: i32, b: Option<Button>) -> MouseEvent {
    MouseEvent {
        t,
        kind,
        x,
        y,
        button: b,
    }
}
fn down(t: u32, x: i32) -> MouseEvent {
    ev(t, EventKind::Down, x, 0, Some(Button::Left))
}
fn up(t: u32) -> MouseEvent {
    ev(t, EventKind::Up, 0, 0, Some(Button::Left))
}

#[test]
fn the_squash_dips_at_the_click_and_is_back_by_120ms() {
    let clicks = [1000u32];
    assert!(
        (squash_at(&clicks, 1000) - SQUASH).abs() < 1e-6,
        "full squash at the instant"
    );
    assert!(
        squash_at(&clicks, 1060) > SQUASH && squash_at(&clicks, 1060) < 1.0,
        "half way back"
    );
    assert_eq!(squash_at(&clicks, 1120), 1.0, "over by SQUASH_MS");
    assert_eq!(squash_at(&clicks, 900), 1.0, "nothing before the click");
    assert_eq!(squash_at(&[], 5), 1.0);
}

#[test]
fn the_ink_drop_runs_once_over_260ms_then_stops() {
    let clicks = [500u32];
    assert!(ink_at(&clicks, 500) < 1e-6, "starts at 0");
    assert!(
        (ink_at(&clicks, 630) - 0.5).abs() < 0.01,
        "half way at 130 ms"
    );
    assert!(ink_at(&clicks, 760) < 0.0, "gone by INK_MS");
    assert!(ink_at(&clicks, 400) < 0.0, "never before the click");
}

#[test]
fn drag_spans_pair_each_press_with_its_release() {
    let events = vec![
        down(100, 5),
        ev(150, EventKind::Move, 9, 9, None),
        up(300),
        down(900, 7),
    ];
    let spans = drag_spans(&events);
    assert_eq!(spans.len(), 2);
    assert_eq!((spans[0].down, spans[0].up, spans[0].x), (100, 300, 5));
    assert_eq!(spans[1].up, u32::MAX, "a press still held has no release");
}

#[test]
fn the_selection_stretches_while_held_and_retracts_after_the_release() {
    let spans = drag_spans(&vec![down(100, 42), up(300)]);
    assert!(
        selection_at(&spans, 50).is_none(),
        "nothing before the press"
    );
    let (a, w) = selection_at(&spans, 100).unwrap();
    assert_eq!(a, [42, 0]);
    assert!(w < 1e-6, "the stretch starts from nothing at the press");
    assert!(
        (selection_at(&spans, 260).unwrap().1 - 1.0).abs() < 1e-6,
        "settled while held"
    );
    assert!(
        (selection_at(&spans, 300).unwrap().1 - 1.0).abs() < 1e-6,
        "still full at the release"
    );
    let mid = selection_at(&spans, 380).unwrap().1;
    assert!(mid > 0.0 && mid < 1.0, "retracting, got {mid}");
    assert!(
        selection_at(&spans, 460).is_none(),
        "gone one MORPH_MS after the release"
    );
}

#[test]
fn the_kind_morph_eases_from_the_previous_shape() {
    let track = CursorTrack {
        samples: vec![(0, CursorType::Arrow), (1000, CursorType::IBeam)],
    };
    assert_eq!(
        kind_morph(&track, 500),
        (CursorType::Arrow, CursorType::Arrow, 1.0)
    );
    let (k, p, m) = kind_morph(&track, 1000);
    assert_eq!((k, p), (CursorType::IBeam, CursorType::Arrow));
    assert!(m < 1e-6, "the change starts at the previous shape");
    assert!(
        (kind_morph(&track, 1160).2 - 1.0).abs() < 1e-6,
        "settled after MORPH_MS"
    );
    assert_eq!(kind_morph(&CursorTrack::default(), 7).2, 1.0);
}

#[test]
fn the_back_is_a_disc_for_an_arrow_and_a_horizontal_pill_over_text() {
    let disc = back_geom(
        CursorType::Arrow,
        CursorType::Arrow,
        1.0,
        [100.0, 100.0],
        40.0,
        None,
        1.0,
        -1.0,
    );
    let (w, h) = (disc.mx[0] - disc.mn[0], disc.mx[1] - disc.mn[1]);
    assert!(
        (w - h).abs() < 1e-4 && (h - 40.0 * BACK_SCALE).abs() < 1e-4,
        "a disc 2.2x the sprite"
    );
    assert!(
        (disc.r - h * 0.5).abs() < 1e-4,
        "corner radius = half the height -> a circle"
    );
    assert!(!disc.ring, "an arrow's click is the ink drop, not the ring");

    let pill = back_geom(
        CursorType::IBeam,
        CursorType::IBeam,
        1.0,
        [100.0, 100.0],
        40.0,
        None,
        1.0,
        -1.0,
    );
    let ph = pill.mx[1] - pill.mn[1];
    assert!(
        (pill.mx[0] - pill.mn[0] - w).abs() < 1e-4,
        "same width as the disc"
    );
    assert!((ph - h * PILL_W).abs() < 1e-4, "0.35x as tall");
    assert!(
        (pill.r - ph * 0.5).abs() < 1e-4,
        "radius = half the short side -> rounded ends"
    );
    assert!(pill.ring, "over text the click hugs the pill instead");

    let mid = back_geom(
        CursorType::IBeam,
        CursorType::Arrow,
        0.5,
        [100.0, 100.0],
        40.0,
        None,
        1.0,
        -1.0,
    );
    let mh = mid.mx[1] - mid.mn[1];
    assert!(mh > ph && mh < h, "half way from disc to pill, got {mh}");
}

#[test]
fn a_held_selection_stretches_the_pill_toward_the_anchor() {
    let at = |w: f32| {
        back_geom(
            CursorType::IBeam,
            CursorType::IBeam,
            1.0,
            [300.0, 100.0],
            40.0,
            Some(([100.0, 100.0], w)),
            1.0,
            -1.0,
        )
    };
    let none = at(0.0);
    let full = at(1.0);
    assert!(
        (full.mn[0] - (100.0 - (none.mx[0] - none.mn[0]) * 0.5)).abs() < 1e-3,
        "the bar reaches the anchor, rounded end included"
    );
    assert!(
        (full.mx[0] - none.mx[0]).abs() < 1e-4,
        "the cursor end does not move"
    );
    assert!(
        (full.mx[1] - full.mn[1] - (none.mx[1] - none.mn[1])).abs() < 1e-4,
        "height unchanged"
    );
    assert!(at(0.5).mn[0] > full.mn[0], "half way is half the reach");
    let arrow = back_geom(
        CursorType::Arrow,
        CursorType::Arrow,
        1.0,
        [300.0, 100.0],
        40.0,
        Some(([100.0, 100.0], 1.0)),
        1.0,
        -1.0,
    );
    assert!(
        arrow.mn[0] > 200.0,
        "an arrow never stretches to the press point"
    );
}

#[test]
fn a_mask_is_the_sprites_alpha_keyed_by_pack_and_kind() {
    let spr = CursorSprite {
        bgra: vec![9, 9, 9, 250, 1, 2, 3, 38, 0, 0, 0, 0],
        w: 3,
        h: 1,
        hot: (0.5, 0.5),
        canvas_h: 3,
    };
    let m = mask_of("crystal", CursorType::Arrow, &spr);
    assert_eq!((m.w, m.h), (3, 1));
    assert_eq!(m.a[0], 255, "the rim is inside");
    assert_eq!(m.a[1], 255, "a 15% body is fully inside too");
    assert_eq!(m.a[2], 0, "and nothing outside the silhouette is");
    assert_ne!(m.key, mask_of("crystal", CursorType::IBeam, &spr).key);
    assert_ne!(m.key, mask_of("aero-glass", CursorType::Arrow, &spr).key);
    assert_eq!(
        m.key,
        mask_of("crystal", CursorType::Arrow, &spr).key,
        "stable across calls"
    );
}

#[test]
fn a_morph_mask_blends_both_silhouettes_and_rekeys_as_it_goes() {
    let a = LensMask {
        key: 11,
        w: 1,
        h: 1,
        a: vec![0],
    };
    let b = LensMask {
        key: 22,
        w: 1,
        h: 1,
        a: vec![255],
    };
    assert_eq!(
        morph_mask(&a, &b, 0.0).a[0],
        0,
        "m=0 is the outgoing silhouette"
    );
    assert_eq!(morph_mask(&a, &b, 1.0).a[0], 255, "m=1 is the incoming one");
    let mid = morph_mask(&a, &b, 0.5);
    assert!(
        (120..=136).contains(&mid.a[0]),
        "half way, got {}",
        mid.a[0]
    );
    assert_eq!(mid.a.len(), (mid.w * mid.h) as usize);
    assert_ne!(mid.key, morph_mask(&a, &b, 0.0).key);
    assert_ne!(
        morph_mask(&a, &b, 1.0).key,
        morph_mask(&b, &a, 1.0).key,
        "direction matters"
    );
    assert_eq!(mid.key, morph_mask(&a, &b, 0.5).key, "stable across calls");
}
