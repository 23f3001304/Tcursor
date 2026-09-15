use super::*;
use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
fn mv(t: u32, x: i32, y: i32) -> MouseEvent {
    MouseEvent {
        t,
        kind: EventKind::Move,
        x,
        y,
        button: None,
    }
}

#[test]
fn center_before_first_event() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let ev = vec![mv(1000, 100, 100)];
    let mut c = Cursor::new(ev, s, 0.6);
    assert_eq!(c.at(0, 16.0), FramePoint { x: 960, y: 540 });
}

#[test]
fn arrives_with_the_recording_not_after_it() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let ev = vec![mv(0, 0, 0), mv(100, 800, 400), mv(2000, 800, 400)];
    for smooth in [0.0, 0.6, 1.0] {
        let mut c = Cursor::new(ev.clone(), s, smooth);
        c.set_idealize(1.0);
        for t in (0..=100).step_by(16) {
            c.at(t, 16.0);
        }
        assert_eq!(
            c.at(100, 16.0),
            FramePoint { x: 800, y: 400 },
            "smooth {smooth}: on the target the instant the hand was"
        );
        assert_eq!(c.at(116, 16.0), FramePoint { x: 800, y: 400 });
    }
}

#[test]
fn idealize_pulls_a_detour_toward_the_click_anchor_line() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let click = |t: u32, x: i32, y: i32| MouseEvent {
        t,
        kind: EventKind::Down,
        x,
        y,
        button: None,
    };
    let mut ev = vec![click(0, 0, 0)];
    for t in (16..1000).step_by(16) {
        let u = t as f32 / 1000.0;
        ev.push(if u < 0.5 {
            mv(t, 0, (1600.0 * u) as i32)
        } else {
            mv(
                t,
                (2000.0 * (u - 0.5)) as i32,
                (800.0 - 1600.0 * (u - 0.5)) as i32,
            )
        });
    }
    ev.push(click(1000, 1000, 0));
    let mut raw = Cursor::new(ev.clone(), s, 0.0);
    let mut ideal = Cursor::new(ev, s, 0.0);
    ideal.set_idealize(1.0);
    for t in (0..=500).step_by(16) {
        raw.at(t, 16.0);
        ideal.at(t, 16.0);
    }
    let (r, i) = (raw.at(500, 16.0), ideal.at(500, 16.0));
    assert!(
        i.y.abs() < r.y.abs(),
        "idealized y {} should hug the anchor line, not the detour {}",
        i.y,
        r.y
    );
    assert_eq!(
        ideal.at(1000, 16.0),
        FramePoint { x: 1000, y: 0 },
        "and the click itself is exact"
    );
}

#[test]
fn no_polish_is_the_raw_interpolation_bit_for_bit() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let ev = vec![
        mv(0, 10, 10),
        mv(100, 300, 40),
        mv(180, 305, 900),
        mv(900, 700, 700),
    ];
    let mut c = Cursor::new(ev.clone(), s, 0.0);
    let mut d = Cursor::new(ev, s, 0.0);
    for t in (0..=1000).step_by(16) {
        d.idx = 0;
        while d.idx + 1 < d.events.len() && d.events[d.idx + 1].t <= t {
            d.idx += 1;
        }
        assert_eq!(c.at(t, 16.0), d.raw_at(t), "t={t}");
    }
}

#[test]
fn a_thrown_cursor_leans_and_a_resting_one_does_not() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let ev = vec![mv(0, 0, 500), mv(600, 1800, 500)];
    let mut c = Cursor::new(ev.clone(), s, 0.0);
    c.set_tilt(1.0);
    for t in (0..=400).step_by(16) {
        c.at(t, 1000.0 / 60.0);
    }
    assert!(
        c.tilt_deg() > 1.0,
        "a fast sweep must lean: {}",
        c.tilt_deg()
    );
    for t in (600..=1400).step_by(16) {
        c.at(t, 1000.0 / 60.0);
    }
    assert!(
        c.tilt_deg().abs() < 0.05,
        "and settle upright again: {}",
        c.tilt_deg()
    );

    let mut off = Cursor::new(ev, s, 0.0);
    for t in (0..=400).step_by(16) {
        off.at(t, 1000.0 / 60.0);
    }
    assert_eq!(off.tilt_deg(), 0.0);
}

#[test]
fn a_cut_snaps_the_lean_away_with_the_position() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let mut c = Cursor::new(vec![mv(0, 0, 500), mv(600, 1800, 500)], s, 0.0);
    c.set_tilt(1.0);
    for t in (0..=400).step_by(16) {
        c.at(t, 1000.0 / 60.0);
    }
    assert!(c.tilt_deg() > 1.0);
    c.reset();
    assert_eq!(c.tilt_deg(), 0.0);
}

#[test]
fn a_rewound_cursor_answers_like_a_fresh_one() {
    let s = ScreenInfo {
        w: 1920,
        h: 1080,
        origin_x: 0,
        origin_y: 0,
    };
    let ev = vec![
        mv(0, 0, 0),
        mv(400, 600, 300),
        mv(900, 600, 300),
        mv(1300, 100, 900),
    ];
    let mut c = Cursor::new(ev.clone(), s, 0.8);
    c.set_idealize(0.5);
    let fresh: Vec<FramePoint> = (0..=1400).step_by(16).map(|t| c.at(t, 16.0)).collect();
    c.reset();
    let again: Vec<FramePoint> = (0..=1400).step_by(16).map(|t| c.at(t, 16.0)).collect();
    assert_eq!(fresh, again);
}
