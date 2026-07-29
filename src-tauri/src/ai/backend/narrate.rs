// Human "what the AI director did + why" lines for the agentic reveal log. Each edit op becomes a
// short past-tense sentence; zooms are correlated back to the click that triggered them so the
// reveal reads like the agent actually understood the recording.
use crate::ai::backend::timeline::region;
use crate::edit::ops::api::EditOp;
use crate::events::model::{EventKind, EventLog};

/// A short narration line for one plan step.
pub fn label_for(op: &EditOp, log: &EventLog, dur_ms: u32) -> String {
    match op {
        EditOp::ClearZooms => "Rethinking your zooms…".into(),
        EditOp::AddZoomFull { at_ms, scale, .. } => zoom_label(*at_ms, *scale, log),
        EditOp::AddZoom { at_ms, .. } => zoom_label(*at_ms, 2.0, log),
        EditOp::SetTrim { in_ms, out_ms } => trim_label(*in_ms, *out_ms, dur_ms),
        _ => "Applied an edit".into(),
    }
}

fn zoom_label(at_ms: u32, scale: f32, log: &EventLog) -> String {
    // Nearest click within 600ms of the zoom start -> narrate the reason ("you clicked <region>").
    let near = log.events.iter()
        .filter(|e| e.kind == EventKind::Down)
        .min_by_key(|e| (e.t as i64 - at_ms as i64).abs())
        .filter(|e| (e.t as i64 - at_ms as i64).abs() <= 600);
    match near {
        Some(e) => format!("Zoomed into the {} — you clicked there · {}", region(e.x, e.y, log.screen.w, log.screen.h), mmss(at_ms)),
        None => format!("Zoomed in at {} · {:.1}×", mmss(at_ms), scale),
    }
}

fn trim_label(in_ms: u32, out_ms: u32, dur_ms: u32) -> String {
    let head = in_ms;
    let tail = if out_ms == 0 || out_ms >= dur_ms { 0 } else { dur_ms - out_ms };
    let s = |ms: u32| format!("{:.1}s", ms as f32 / 1000.0);
    match (head > 0, tail > 0) {
        (true, true) => format!("Trimmed {} off the start and {} off the end", s(head), s(tail)),
        (true, false) => format!("Trimmed {} of dead air off the start", s(head)),
        (false, true) => format!("Trimmed {} of dead air off the end", s(tail)),
        (false, false) => "Kept the full clip".into(),
    }
}

fn mmss(ms: u32) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{EventKind, MouseEvent, ScreenInfo};

    fn log_with_click(t: u32, x: i32, y: i32) -> EventLog {
        EventLog { started_unix_ms: 0, screen: ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 },
            events: vec![MouseEvent { t, kind: EventKind::Down, x, y, button: None }] }
    }

    #[test]
    fn zoom_near_a_click_names_the_region_and_time() {
        let s = label_for(&EditOp::AddZoomFull { at_ms: 3100, dur_ms: 1500, scale: 2.0 }, &log_with_click(3000, 100, 100), 10_000);
        assert!(s.contains("top-left") && s.contains("clicked"), "{}", s);
        assert!(s.contains("0:03"), "{}", s);
    }

    #[test]
    fn zoom_far_from_clicks_falls_back_to_time_and_scale() {
        let s = label_for(&EditOp::AddZoomFull { at_ms: 68_000, dur_ms: 1500, scale: 2.4 }, &log_with_click(0, 100, 100), 90_000);
        assert!(s.contains("1:08") && s.contains("2.4"), "{}", s);
    }

    #[test]
    fn trim_head_and_tail_both_named() {
        let s = trim_label(2000, 8000, 10_000);
        assert!(s.contains("start") && s.contains("end"), "{}", s);
    }

    #[test]
    fn trim_out_zero_means_head_only() {
        let s = trim_label(2000, 0, 10_000);
        assert!(s.contains("start") && !s.contains("end"), "{}", s);
    }
}
