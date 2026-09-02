// Tests for ai::backend::timeline, split into their own file so timeline.rs stays under the size limit.
use super::*;
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::events::model::{EventKind, EventLog, MouseEvent, ScreenInfo};

fn scr() -> ScreenInfo { ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 } }
fn ev(t: u32, kind: EventKind, x: i32, y: i32) -> MouseEvent {
    MouseEvent { t, kind, x, y, button: None }
}
fn make_log() -> EventLog {
    EventLog { started_unix_ms: 0, screen: scr(), events: vec![
        ev(500, EventKind::Down, 100, 100), ev(500, EventKind::Move, 100, 100),
        ev(3500, EventKind::Down, 960, 540), ev(3500, EventKind::Move, 960, 540),
    ]}
}

#[test]
fn region_corners_and_center() {
    assert_eq!(region(0, 0, 1920, 1080), "top-left");
    assert_eq!(region(1919, 0, 1920, 1080), "top-right");
    assert_eq!(region(0, 1079, 1920, 1080), "bottom-left");
    assert_eq!(region(1919, 1079, 1920, 1080), "bottom-right");
    assert_eq!(region(960, 540, 1920, 1080), "center");
    assert_eq!(region(0, 540, 1920, 1080), "left");
    assert_eq!(region(1919, 540, 1920, 1080), "right");
    assert_eq!(region(960, 0, 1920, 1080), "top");
    assert_eq!(region(960, 1079, 1920, 1080), "bottom");
}

#[test]
fn serialize_contains_expected_lines() {
    let out = serialize(&make_log(),
        &[ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Camera) }],
        &CursorTrack::default(), &[1000u32, 1200, 1400], 5000, 0);
    assert!(out.starts_with("clip 5.0s, screen 1920x1080"), "{}", out);
    assert!(out.contains("0.5s click (100,100) top-left"), "{}", out);
    assert!(out.contains("3.5s click (960,540) center"), "{}", out);
    assert!(out.contains("layout -> camera"), "{}", out);
    assert!(out.contains("1.0-1.4s typing"), "{}", out);
    assert!(out.contains("0.5-3.5s idle"), "{}", out);
}

#[test]
fn time_ordered() {
    let out = serialize(&make_log(),
        &[ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Screen) }],
        &CursorTrack::default(), &[1000u32, 1100], 5000, 0);
    let times: Vec<f64> = out.lines().skip(1).filter_map(|l|
        l.split('s').next().and_then(|s| s.split('-').next()?.parse().ok())
    ).collect();
    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(times, sorted);
}

#[test]
fn deterministic() {
    let (log, actions, cursor, typing) = (make_log(),
        vec![ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Camera) }],
        CursorTrack::default(), vec![1000u32, 1200]);
    assert_eq!(serialize(&log, &actions, &cursor, &typing, 5000, -800),
               serialize(&log, &actions, &cursor, &typing, 5000, -800));
}

#[test]
fn ibeam_span_emitted() {
    let log = EventLog { started_unix_ms: 0, screen: scr(), events: vec![] };
    let cursor = CursorTrack { samples: vec![(1000, CursorType::IBeam), (2000, CursorType::Arrow)] };
    let out = serialize(&log, &[], &cursor, &[], 5000, 0);
    assert!(out.contains("text field"), "{}", out);
}

/// The whole point of the task: with `events_ms=0, video_start=800` (`shift=-800`, the
/// ~0.8s real-recording offset), a click recorded at raw event t=3100 must serialize at
/// its OUTPUT-clock time (2.3s), not its raw event-clock time (3.1s) - the director's ops
/// land as output-time zooms, so its transcript must reason on that same clock.
#[test]
fn click_timestamps_shift_onto_the_output_clock() {
    let log = EventLog { started_unix_ms: 0, screen: scr(),
        events: vec![ev(3100, EventKind::Down, 100, 100), ev(3100, EventKind::Move, 100, 100)] };
    let out = serialize(&log, &[], &CursorTrack::default(), &[], 5000, -800);
    assert!(out.contains("2.3s click"), "{}", out);
    assert!(!out.contains("3.1s click"), "{}", out);
}

/// H1: a secondary monitor placed right of primary has `origin_x = 1920` (virtual-desktop
/// coordinates). A click at the dead center of THAT screen is virtual-desktop x=2880 - the
/// transcript must print/classify the SCREEN-LOCAL point (960,540 -> "center"), not the raw
/// virtual-desktop one (2880,540), which would misclassify as "right" and print an
/// off-screen coordinate against the `screen 1920x1080` header line.
#[test]
fn click_coords_convert_through_the_screen_origin_before_regioning() {
    let log = EventLog { started_unix_ms: 0,
        screen: ScreenInfo { w: 1920, h: 1080, origin_x: 1920, origin_y: 0 },
        events: vec![ev(1000, EventKind::Down, 2880, 540)] };
    let out = serialize(&log, &[], &CursorTrack::default(), &[], 5000, 0);
    assert!(out.contains("(960,540) center"), "{}", out);
    assert!(!out.contains("(2880,540)"), "{}", out);
}

/// A monitor placed LEFT of primary has a NEGATIVE origin_x - without the origin subtraction
/// every click's raw x is negative, so `region()`'s `< w/3` branch always wins and every
/// single click serializes as some flavor of "left", regardless of where on the screen it
/// actually landed.
#[test]
fn click_coords_convert_correctly_for_a_monitor_left_of_primary() {
    let log = EventLog { started_unix_ms: 0,
        screen: ScreenInfo { w: 1920, h: 1080, origin_x: -1920, origin_y: 0 },
        events: vec![ev(1000, EventKind::Down, -960, 540)] }; // virtual-desktop x=-960 -> screen-local x=960 (center column)
    let out = serialize(&log, &[], &CursorTrack::default(), &[], 5000, 0);
    assert!(out.contains("(960,540) center"), "{}", out);
}

/// Layout switches, typing spans and idle windows all shift by the SAME amount as clicks -
/// every timestamp in the transcript must agree on one clock. The clip duration line does
/// NOT shift: the caller already passes the true output-clock duration.
#[test]
fn layout_typing_and_idle_timestamps_shift_identically() {
    let out = serialize(&make_log(),
        &[ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Camera) }],
        &CursorTrack::default(), &[1000u32, 1200, 1400], 5000, -800);
    assert!(out.starts_with("clip 5.0s"), "duration line must not shift: {}", out);
    assert!(out.contains("0.0s click (100,100) top-left"), "500ms - 800ms clamps to 0: {}", out);
    assert!(out.contains("2.7s click (960,540) center"), "{}", out);
    assert!(out.contains("1.2s layout -> camera"), "{}", out);
    assert!(out.contains("0.2-0.6s typing"), "{}", out);
    assert!(out.contains("0.0-2.7s idle"), "{}", out);
}
