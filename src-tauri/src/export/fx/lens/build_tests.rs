use super::*;
use crate::events::model::{Button, EventKind, MouseEvent};
use crate::events::track::cursortype::CursorTrack;
use crate::export::cursor::cursordraw::CursorSprite;
use crate::export::fx::lens::{drag_spans, mask_of, LensMask};
use crate::export::types::RectF;
use crate::settings::cursor::CursorStyle;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

fn sprite() -> CursorSprite {
    CursorSprite {
        bgra: vec![255; 4 * 4 * 4],
        w: 4,
        h: 4,
        hot: (0.0, 0.0),
        canvas_h: 4,
    }
}

fn prep(glass: bool, kind_at: Vec<(u32, CursorType)>, events: &[MouseEvent]) -> CursorPrep {
    let mut set = HashMap::new();
    for k in [CursorType::Arrow, CursorType::IBeam] {
        set.insert(k, sprite());
    }
    let masks: HashMap<CursorType, Arc<LensMask>> = if glass {
        set.iter()
            .map(|(k, s)| (*k, Arc::new(mask_of("glassy", *k, s))))
            .collect()
    } else {
        HashMap::new()
    };
    CursorPrep {
        set,
        track: CursorTrack { samples: kind_at },
        click_ms: events
            .iter()
            .filter(|e| e.kind == EventKind::Down)
            .map(|e| e.t)
            .collect(),
        recent: VecDeque::new(),
        busy: None,
        busy_frames: Vec::new(),
        glass,
        masks,
        drags: drag_spans(events),
    }
}

fn panel() -> Panel {
    Panel {
        rect: RectF {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        },
        radius: 0.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    }
}

fn info() -> ScreenInfo {
    ScreenInfo {
        w: 200,
        h: 200,
        origin_x: 0,
        origin_y: 0,
    }
}

fn frame<'a>(p: &'a Panel, i: &'a ScreenInfo, ev_t: u32) -> LensFrame<'a> {
    LensFrame {
        cur: FramePoint { x: 100, y: 100 },
        cam: Camera {
            cx: 100.0,
            cy: 100.0,
            scale: 1.0,
        },
        ow: 200,
        oh: 200,
        screen: p,
        inset_w: 200.0,
        info: i,
        src: crate::export::coordmap::full_src(200, 200),
        ev_t,
        out_t: ev_t,
        tilt_deg: 0.0,
    }
}

fn settings(back: CursorBack, pack_glass: bool) -> CursorSettings {
    let _ = pack_glass;
    CursorSettings {
        style: CursorStyle::Enhanced,
        back,
        click_bounce: false,
        ..Default::default()
    }
}

#[test]
fn a_plain_pack_with_no_back_asks_for_nothing() {
    let cp = prep(false, vec![], &[]);
    assert!(!wants_lens(
        Some(&cp),
        &settings(CursorBack::None, false),
        false
    ));
    let (p, i) = (panel(), info());
    assert!(lenses_at(
        &cp,
        &settings(CursorBack::None, false),
        frame(&p, &i, 0),
        false
    )
    .is_none());
}

#[test]
fn a_glass_pack_places_the_lens_on_the_sprites_own_box() {
    let cp = prep(true, vec![], &[]);
    let (p, i) = (panel(), info());
    let l = lenses_at(
        &cp,
        &settings(CursorBack::None, false),
        frame(&p, &i, 0),
        false,
    )
    .unwrap();
    let g = l.glass.expect("a glass pack gets a lens");
    assert!(l.back.is_none(), "the back is a separate opt-in");
    assert!(
        (g.cbox[2] - 6.6).abs() < 0.01 && (g.cbox[3] - 6.6).abs() < 0.01,
        "box {:?}",
        g.cbox
    );
    assert!((g.cbox[0] - 103.3).abs() < 0.01 && (g.cbox[1] - 103.3).abs() < 0.01);
    assert_eq!(g.angle, 0.0, "no busy spin on an arrow");
    assert_eq!(g.squash, 1.0);
    assert!(g.ink < 0.0, "no click, no ink");
}

#[test]
fn the_plain_os_cursor_gets_no_glass_at_all() {
    let cp = prep(true, vec![], &[]);
    let (p, i) = (panel(), info());
    assert!(!wants_lens(
        Some(&cp),
        &settings(CursorBack::Glass, true),
        true
    ));
    assert!(lenses_at(
        &cp,
        &settings(CursorBack::Glass, true),
        frame(&p, &i, 0),
        true
    )
    .is_none());
}

#[test]
fn a_click_squashes_the_back_and_drops_ink_at_the_cursor() {
    let ev = vec![MouseEvent {
        t: 500,
        kind: EventKind::Down,
        x: 100,
        y: 100,
        button: Some(Button::Left),
    }];
    let cp = prep(false, vec![], &ev);
    let (p, i) = (panel(), info());
    let b = lenses_at(
        &cp,
        &settings(CursorBack::Glass, false),
        frame(&p, &i, 500),
        false,
    )
    .unwrap()
    .back
    .unwrap();
    assert!((b.squash - crate::export::fx::lens::SQUASH).abs() < 1e-6);
    assert!(b.ink >= 0.0 && b.ink < 0.01, "the drop starts at the click");
    assert_eq!(
        b.ink_at,
        [(b.mn[0] + b.mx[0]) * 0.5, (b.mn[1] + b.mx[1]) * 0.5]
    );
    let later = lenses_at(
        &cp,
        &settings(CursorBack::Glass, false),
        frame(&p, &i, 800),
        false,
    )
    .unwrap()
    .back
    .unwrap();
    assert_eq!(later.squash, 1.0);
    assert!(later.ink < 0.0);
}

#[test]
fn over_text_the_back_is_a_pill_that_stretches_to_the_press_point() {
    let ev = vec![MouseEvent {
        t: 0,
        kind: EventKind::Down,
        x: 40,
        y: 100,
        button: Some(Button::Left),
    }];
    let cp = prep(false, vec![(0, CursorType::IBeam)], &ev);
    let (p, i) = (panel(), info());
    let held = lenses_at(
        &cp,
        &settings(CursorBack::Glass, false),
        frame(&p, &i, 400),
        false,
    )
    .unwrap()
    .back
    .unwrap();
    assert!(held.ring, "over text the click is a ring on the pill");
    assert!(
        held.mn[0] < 45.0,
        "the bar reaches back to the press at x=40, got {}",
        held.mn[0]
    );
    assert!(held.mx[0] > 100.0, "and still covers the cursor at x=100");
}

#[test]
fn a_glass_pack_with_a_glass_back_gets_both_shapes() {
    let cp = prep(true, vec![], &[]);
    let (p, i) = (panel(), info());
    let l = lenses_at(
        &cp,
        &settings(CursorBack::Glass, true),
        frame(&p, &i, 0),
        false,
    )
    .unwrap();
    assert!(l.glass.is_some() && l.back.is_some());
}

#[test]
fn a_faded_out_screen_panel_has_no_cursor_and_so_no_lens() {
    let cp = prep(true, vec![], &[]);
    let hidden = Panel {
        alpha: 0.0,
        ..panel()
    };
    let i = info();
    assert!(lenses_at(
        &cp,
        &settings(CursorBack::Glass, true),
        frame(&hidden, &i, 0),
        false
    )
    .is_none());
}
