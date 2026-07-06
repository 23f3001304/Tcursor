use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::events::model::{EventKind, EventLog};

fn region(x: i32, y: i32, w: u32, h: u32) -> &'static str {
    let col = if x < (w / 3) as i32 { 0 } else if x < (2 * w / 3) as i32 { 1 } else { 2 };
    let row = if y < (h / 3) as i32 { 0 } else if y < (2 * h / 3) as i32 { 1 } else { 2 };
    match (row, col) {
        (0, 0) => "top-left", (0, 1) => "top", (0, 2) => "top-right",
        (1, 0) => "left",     (1, 1) => "center", (1, 2) => "right",
        _ => match col { 0 => "bottom-left", 1 => "bottom", _ => "bottom-right" },
    }
}

fn fmt(ms: u32) -> String { format!("{:.1}", ms as f64 / 1000.0) }
struct Moment { t_ms: u32, line: String }
fn mo(t_ms: u32, line: String) -> Moment { Moment { t_ms, line } }

pub fn serialize(
    log: &EventLog, actions: &[ActionEvent], cursor: &CursorTrack,
    typing: &[u32], dur_ms: u32,
) -> String {
    let (w, h) = (log.screen.w, log.screen.h);
    let mut m: Vec<Moment> = Vec::new();

    for ev in &log.events {
        if ev.kind == EventKind::Down {
            m.push(mo(ev.t, format!("{}s click ({},{}) {}", fmt(ev.t), ev.x, ev.y, region(ev.x, ev.y, w, h))));
        }
    }

    for act in actions {
        if let ActionKind::SetLayout(id) = act.kind {
            m.push(mo(act.t, format!("{}s layout -> {}", fmt(act.t), format!("{:?}", id).to_lowercase())));
        }
    }

    if !typing.is_empty() {
        let (mut s, mut e) = (typing[0], typing[0]);
        for &ms in &typing[1..] {
            if ms - e < 1000 { e = ms; }
            else {
                m.push(mo(s, format!("{}-{}s typing", fmt(s), fmt(e))));
                s = ms; e = ms;
            }
        }
        m.push(mo(s, format!("{}-{}s typing", fmt(s), fmt(e))));
    }

    let smp = &cursor.samples;
    let mut ib: Option<u32> = None;
    for i in 0..smp.len() {
        let (t, ct) = smp[i];
        if ct == CursorType::IBeam && ib.is_none() { ib = Some(t); }
        else if ct != CursorType::IBeam {
            if let Some(start) = ib.take() {
                let end = if i > 0 { smp[i - 1].0 } else { start };
                m.push(mo(start, format!("{}-{}s text field", fmt(start), fmt(end))));
            }
        }
    }
    if let Some(start) = ib {
        let end = smp.last().map(|s| s.0).unwrap_or(start);
        m.push(mo(start, format!("{}-{}s text field", fmt(start), fmt(end))));
    }

    let times: Vec<u32> = log.events.iter().map(|e| e.t).collect();
    for w in times.windows(2) {
        if w[1] - w[0] >= 2000 {
            m.push(mo(w[0], format!("{}-{}s idle", fmt(w[0]), fmt(w[1]))));
        }
    }

    m.sort_by_key(|x| x.t_ms);
    const CAP: usize = 120;
    let capped = m.len() > CAP;
    if capped { m.truncate(CAP); }

    let mut lines = vec![format!("clip {}s, screen {}x{}", fmt(dur_ms), log.screen.w, log.screen.h)];
    for x in &m { lines.push(x.line.clone()); }
    if capped { lines.push(format!("(capped at {} moments)", CAP)); }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
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
            &CursorTrack::default(), &[1000u32, 1200, 1400], 5000);
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
            &CursorTrack::default(), &[1000u32, 1100], 5000);
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
        assert_eq!(serialize(&log, &actions, &cursor, &typing, 5000),
                   serialize(&log, &actions, &cursor, &typing, 5000));
    }

    #[test]
    fn ibeam_span_emitted() {
        let log = EventLog { started_unix_ms: 0, screen: scr(), events: vec![] };
        let cursor = CursorTrack { samples: vec![(1000, CursorType::IBeam), (2000, CursorType::Arrow)] };
        let out = serialize(&log, &[], &cursor, &[], 5000);
        assert!(out.contains("text field"), "{}", out);
    }
}
