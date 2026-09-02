// Human "what the AI director did + why" lines for the agentic reveal log. Each edit op becomes a
// short past-tense sentence; zooms are correlated back to the click that triggered them so the
// reveal reads like the agent actually understood the recording.
use crate::ai::backend::timeline::{region, sh};
use crate::edit::ops::api::EditOp;
use crate::events::model::{EventKind, EventLog};
use crate::export::coordmap::to_frame;

/// A short narration line for one plan step. `shift`: ms to ADD to `log`'s raw EVENT-clock
/// timestamps to land on the OUTPUT clock (`edit::seed::output_shift` / `ai::commands::build_plan`'s
/// recipe) - the SAME clock `op`'s `at_ms` is already on (M1: `ai::commands::ai_plan` builds its
/// transcript, and therefore its ops, on the output clock via `timeline::serialize`'s own `shift`
/// parameter; without passing it here too, `zoom_label` compared an output-clock `at_ms` against
/// raw event-clock click timestamps - off by the ~800ms capture-warmup lead on a real recording,
/// which is enough to miss the correlating click entirely or attribute an unrelated one).
pub fn label_for(op: &EditOp, log: &EventLog, dur_ms: u32, shift: i64) -> String {
    match op {
        EditOp::ClearZooms => "Rethinking your zooms…".into(),
        EditOp::AddZoomFull { at_ms, scale, .. } => zoom_label(*at_ms, *scale, log, shift),
        EditOp::AddZoom { at_ms, .. } => zoom_label(*at_ms, 2.0, log, shift),
        EditOp::SetTrim { in_ms, out_ms } => trim_label(*in_ms, *out_ms, dur_ms),
        _ => "Applied an edit".into(),
    }
}

fn zoom_label(at_ms: u32, scale: f32, log: &EventLog, shift: i64) -> String {
    // Nearest click within 600ms of the zoom start -> narrate the reason ("you clicked <region>").
    // Compare on the OUTPUT clock (both sides): `at_ms` already is, `e.t` is shifted here via the
    // same `sh` helper `timeline::serialize` used to build the transcript this op was planned from.
    let near = log.events.iter()
        .filter(|e| e.kind == EventKind::Down)
        .min_by_key(|e| (sh(e.t, shift) as i64 - at_ms as i64).abs())
        .filter(|e| (sh(e.t, shift) as i64 - at_ms as i64).abs() <= 600);
    match near {
        // H1: `e.x`/`e.y` are virtual-desktop coordinates - convert through the screen origin
        // (same as `timeline::serialize`) before naming a region, so the narration always agrees
        // with what the transcript itself showed the model.
        Some(e) => {
            let p = to_frame(&log.screen, e.x, e.y);
            format!("Zoomed into the {} — you clicked there · {}", region(p.x, p.y, log.screen.w, log.screen.h), mmss(at_ms))
        }
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
        let s = label_for(&EditOp::AddZoomFull { at_ms: 3100, dur_ms: 1500, scale: 2.0 }, &log_with_click(3000, 100, 100), 10_000, 0);
        assert!(s.contains("top-left") && s.contains("clicked"), "{}", s);
        assert!(s.contains("0:03"), "{}", s);
    }

    #[test]
    fn zoom_far_from_clicks_falls_back_to_time_and_scale() {
        let s = label_for(&EditOp::AddZoomFull { at_ms: 68_000, dur_ms: 1500, scale: 2.4 }, &log_with_click(0, 100, 100), 90_000, 0);
        assert!(s.contains("1:08") && s.contains("2.4"), "{}", s);
    }

    /// M1, the one-clock correlation fixture: `events_ms=0, video_start=800` (shift=-800, a
    /// representative real-recording capture-warmup lead). A click recorded at raw event t=3100
    /// lands on the OUTPUT clock at 2300ms. A zoom the director placed at `at_ms=2300` must find
    /// it (comparing on the SAME clock, |2300-2300|=0 <= 600) - comparing raw `e.t=3100` against
    /// `at_ms=2300` (the pre-fix bug) would have missed it by 800ms, well outside the window, and
    /// silently fallen back to the generic time-only label.
    #[test]
    fn zoom_correlates_against_the_output_clock_the_zoom_itself_is_on() {
        let log = log_with_click(3100, 100, 100); // raw EVENT-clock timestamp
        let s = label_for(&EditOp::AddZoomFull { at_ms: 2300, dur_ms: 1500, scale: 2.0 }, &log, 10_000, -800);
        assert!(s.contains("top-left") && s.contains("clicked"), "{}", s);
    }

    /// Combines M1 (one-clock correlation) with H1 (screen-origin conversion): events_ms=0,
    /// video_start=800 (shift=-800) AND a secondary monitor at origin_x=1920. A click at raw
    /// t=3100, virtual-desktop (2880,540) is: output-clock 2300ms, screen-local (960,540) =
    /// "center" - both fixes must compose correctly for the narration to name the right region.
    #[test]
    fn zoom_label_combines_output_clock_correlation_with_screen_local_coords() {
        let log = EventLog { started_unix_ms: 0,
            screen: ScreenInfo { w: 1920, h: 1080, origin_x: 1920, origin_y: 0 },
            events: vec![MouseEvent { t: 3100, kind: EventKind::Down, x: 2880, y: 540, button: None }] };
        let s = label_for(&EditOp::AddZoomFull { at_ms: 2300, dur_ms: 1500, scale: 2.0 }, &log, 10_000, -800);
        assert!(s.contains("center") && s.contains("clicked"), "{}", s);
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
