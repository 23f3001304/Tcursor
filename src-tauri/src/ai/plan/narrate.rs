use crate::ai::plan::schema::ProposalKind;
use crate::ai::plan::transcript::{region, sh};
use crate::events::model::{EventKind, EventLog};
use crate::export::coordmap::to_frame;

pub fn why_for(kind: ProposalKind, at_ms: u32, log: &EventLog, shift: i64) -> String {
    let near = near_click(at_ms, log, shift);
    match kind {
        ProposalKind::Zoom => match near {
            Some(r) => format!("you click the {r} there"),
            None => format!("a moment worth a closer look at {}", mmss(at_ms)),
        },
        ProposalKind::Spotlight => match near {
            Some(r) => format!("you point at the {r}"),
            None => format!("the eye belongs here at {}", mmss(at_ms)),
        },
        ProposalKind::Layout => "you turn to the camera".into(),
        ProposalKind::Trim => "dead air at the edges of the clip".into(),
        ProposalKind::Cut => format!("dead air in the middle at {}", mmss(at_ms)),
        ProposalKind::Speed => format!("a long stretch of typing at {}", mmss(at_ms)),
    }
}

fn near_click(at_ms: u32, log: &EventLog, shift: i64) -> Option<&'static str> {
    let e = log
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Down)
        .min_by_key(|e| (sh(e.t, shift) as i64 - at_ms as i64).abs())
        .filter(|e| (sh(e.t, shift) as i64 - at_ms as i64).abs() <= 600)?;
    let p = to_frame(&log.screen, e.x, e.y);
    Some(region(p.x, p.y, log.screen.w, log.screen.h))
}

fn mmss(ms: u32) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{MouseEvent, ScreenInfo};

    fn log_with_click(t: u32, x: i32, y: i32) -> EventLog {
        EventLog {
            started_unix_ms: 0,
            screen: ScreenInfo {
                w: 1920,
                h: 1080,
                origin_x: 0,
                origin_y: 0,
            },
            events: vec![MouseEvent {
                t,
                kind: EventKind::Down,
                x,
                y,
                button: None,
            }],
        }
    }

    #[test]
    fn a_zoom_near_a_click_names_the_region_it_happened_in() {
        let s = why_for(ProposalKind::Zoom, 3100, &log_with_click(3000, 100, 100), 0);
        assert_eq!(s, "you click the top-left there");
    }

    #[test]
    fn a_zoom_far_from_any_click_falls_back_to_the_time() {
        let s = why_for(ProposalKind::Zoom, 68_000, &log_with_click(0, 100, 100), 0);
        assert!(s.contains("1:08"), "{}", s);
    }

    #[test]
    fn correlation_happens_on_the_output_clock_the_proposal_is_on() {
        let s = why_for(
            ProposalKind::Zoom,
            2300,
            &log_with_click(3100, 100, 100),
            -800,
        );
        assert_eq!(s, "you click the top-left there");
    }

    #[test]
    fn a_click_on_a_second_monitor_is_named_in_screen_local_coordinates() {
        let log = EventLog {
            started_unix_ms: 0,
            screen: ScreenInfo {
                w: 1920,
                h: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
            events: vec![MouseEvent {
                t: 3100,
                kind: EventKind::Down,
                x: 2880,
                y: 540,
                button: None,
            }],
        };
        assert_eq!(
            why_for(ProposalKind::Spotlight, 2300, &log, -800),
            "you point at the center"
        );
    }

    #[test]
    fn every_kind_has_something_honest_to_say() {
        let log = log_with_click(0, 100, 100);
        for k in [
            ProposalKind::Zoom,
            ProposalKind::Layout,
            ProposalKind::Spotlight,
            ProposalKind::Trim,
            ProposalKind::Cut,
            ProposalKind::Speed,
        ] {
            let s = why_for(k, 42_000, &log, 0);
            assert!(!s.is_empty() && !s.contains('\n'), "{:?} -> {}", k, s);
        }
    }
}
