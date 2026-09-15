use super::*;
use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};

const S: ScreenInfo = ScreenInfo {
    w: 1920,
    h: 1080,
    origin_x: 0,
    origin_y: 0,
};
fn mv(t: u32, x: i32, y: i32) -> MouseEvent {
    MouseEvent {
        t,
        kind: EventKind::Move,
        x,
        y,
        button: None,
    }
}
fn down(t: u32, x: i32, y: i32) -> MouseEvent {
    MouseEvent {
        t,
        kind: EventKind::Down,
        x,
        y,
        button: Some(Button::Left),
    }
}

fn leg(
    out: &mut Vec<MouseEvent>,
    t0: u32,
    t1: u32,
    t2: u32,
    a: (i32, i32),
    b: (i32, i32),
    detour: impl Fn(f32) -> (f32, f32),
) {
    let mut t = t0;
    while t < t1 {
        out.push(mv(t, a.0 + (t % 3) as i32 - 1, a.1));
        t += 8;
    }
    let mut t = t1;
    while t < t2 {
        let u = (t - t1) as f32 / (t2 - t1) as f32;
        let (dx, dy) = detour(u);
        out.push(mv(
            t,
            (a.0 as f32 + (b.0 - a.0) as f32 * u + dx).round() as i32,
            (a.1 as f32 + (b.1 - a.1) as f32 * u + dy).round() as i32,
        ));
        t += 8;
    }
    out.push(mv(t2, b.0, b.1));
}

fn scene() -> Vec<MouseEvent> {
    let mut ev = Vec::new();
    leg(&mut ev, 0, 500, 1000, (100, 100), (900, 400), |u| {
        (0.0, -200.0 * (u * std::f32::consts::PI).sin())
    });
    ev.push(down(200, 100, 100));
    leg(&mut ev, 1008, 1500, 2000, (900, 400), (300, 800), |u| {
        (60.0 * (u * 40.0).sin(), 0.0)
    });
    ev.push(down(1200, 900, 400));
    ev.sort_by_key(|e| e.t);
    ev
}

fn at(m: &mut PathModel, t: u32, smooth: f32, ideal: f32) -> (f32, f32) {
    m.at(t, smooth, ideal).unwrap()
}

#[test]
fn rests_and_clicks_are_the_recording_verbatim_at_any_polish() {
    let ev = scene();
    let mut m = PathModel::new(&ev, &S);
    for (smooth, ideal) in [(0.0, 0.0), (0.6, 0.0), (1.0, 1.0), (0.0, 1.0)] {
        assert_eq!(
            at(&mut m, 200, smooth, ideal),
            (100.0, 100.0),
            "click at A, polish {smooth}/{ideal}"
        );
        assert_eq!(
            at(&mut m, 1200, smooth, ideal),
            (900.0, 400.0),
            "click at B, polish {smooth}/{ideal}"
        );
        assert_eq!(
            at(&mut m, 1000, smooth, ideal),
            (900.0, 400.0),
            "arrival at B is on time, polish {smooth}/{ideal}"
        );
        assert_eq!(
            at(&mut m, 2000, smooth, ideal),
            (300.0, 800.0),
            "arrival at C is on time, polish {smooth}/{ideal}"
        );
        let (x, y) = at(&mut m, 300, smooth, ideal);
        assert!(
            (x - 100.0).abs() <= 1.0 && y == 100.0,
            "inside a rest the cursor is the raw hand: {x},{y}"
        );
    }
}

#[test]
fn idealize_straightens_the_detour_and_smoothness_alone_does_not() {
    let ev = scene();
    let mut m = PathModel::new(&ev, &S);
    let chord_dist = |p: (f32, f32)| {
        let (ax, ay, bx, by) = (100.0f32, 100.0f32, 900.0f32, 400.0f32);
        ((bx - ax) * (ay - p.1) - (ax - p.0) * (by - ay)).abs()
            / ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
    };
    let raw = at(&mut m, 750, 0.0, 0.0);
    let ideal = at(&mut m, 750, 0.0, 1.0);
    let half = at(&mut m, 750, 0.0, 0.5);
    assert!(
        chord_dist(raw) > 150.0,
        "the raw detour is well off the chord: {raw:?}"
    );
    assert!(
        chord_dist(ideal) < 1.0,
        "fully idealized rides the chord: {ideal:?}"
    );
    assert!(
        (chord_dist(half) - chord_dist(raw) / 2.0).abs() < 5.0,
        "half idealized is half way: {half:?}"
    );
    let smooth_only = at(&mut m, 750, 1.0, 0.0);
    assert!(
        chord_dist(smooth_only) > 120.0,
        "smoothness keeps the route, it does not straighten it: {smooth_only:?}"
    );
}

#[test]
fn smoothness_irons_out_jitter_and_hesitation_but_pins_both_ends() {
    let ev = scene();
    let mut m = PathModel::new(&ev, &S);
    let jerk = |m: &mut PathModel, smooth: f32| {
        let xs: Vec<f32> = (1500..=2000)
            .step_by(16)
            .map(|t| at(m, t, smooth, 0.0).0)
            .collect();
        xs.windows(3)
            .map(|w| (w[2] - 2.0 * w[1] + w[0]).abs())
            .sum::<f32>()
    };
    let (rough, glassy) = (jerk(&mut m, 0.0), jerk(&mut m, 1.0));
    assert!(
        glassy < rough * 0.5,
        "smooth 1 must cut the zig-zag: {rough} -> {glassy}"
    );
    assert_eq!(at(&mut m, 1500, 1.0, 0.0), (900.0, 400.0));
    assert_eq!(at(&mut m, 2000, 1.0, 0.0), (300.0, 800.0));
}

#[test]
fn a_click_mid_flight_is_still_hit_exactly() {
    let mut ev = Vec::new();
    leg(&mut ev, 0, 300, 1000, (100, 100), (1100, 600), |u| {
        (0.0, 200.0 * (u * std::f32::consts::PI).sin())
    });
    ev.push(down(504, 604, 352));
    ev.sort_by_key(|e| e.t);
    let mut m = PathModel::new(&ev, &S);
    assert_eq!(at(&mut m, 504, 1.0, 1.0), (604.0, 352.0));
    assert_eq!(at(&mut m, 504, 0.0, 0.0), (604.0, 352.0));
}

#[test]
fn a_gap_with_no_events_is_a_rest_not_a_glide() {
    let ev = vec![
        mv(0, 100, 100),
        mv(1000, 140, 100),
        mv(1008, 180, 100),
        mv(1016, 220, 100),
    ];
    let mut m = PathModel::new(&ev, &S);
    assert_eq!(
        at(&mut m, 500, 1.0, 1.0),
        (100.0, 100.0),
        "held where it was, not half way to the next sample"
    );
    assert_eq!(
        at(&mut m, 983, 1.0, 1.0),
        (100.0, 100.0),
        "still held until the move begins (LEAD_MS before the first sample that left)"
    );
    assert_eq!(at(&mut m, 1016, 1.0, 1.0), (220.0, 100.0));
    assert_eq!(
        at(&mut m, 5000, 1.0, 1.0),
        (220.0, 100.0),
        "the last sample holds to the end"
    );
}

#[test]
fn no_polish_replays_the_recordings_own_timing() {
    let ev = vec![
        mv(0, 0, 0),
        mv(200, 0, 0),
        mv(208, 100, 0),
        mv(216, 200, 0),
        mv(256, 200, 0),
        mv(264, 300, 0),
        mv(272, 400, 0),
        mv(500, 400, 0),
    ];
    let mut m = PathModel::new(&ev, &S);
    let (x, _) = at(&mut m, 250, 0.0, 0.0);
    assert!(
        (x - 200.0).abs() < 1.0,
        "smooth 0 keeps the hesitation: {x}"
    );
    let (xs, _) = at(&mut m, 250, 1.0, 0.0);
    assert!(xs > 250.0, "smooth 1 eases straight through it: {xs}");
}

#[test]
fn an_empty_log_answers_none() {
    let mut m = PathModel::new(&[], &S);
    assert_eq!(m.at(0, 0.5, 0.5), None);
}
