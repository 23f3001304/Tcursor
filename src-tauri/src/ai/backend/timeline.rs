use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::events::model::{EventKind, EventLog};
use crate::export::coordmap::to_frame;

pub(crate) fn region(x: i32, y: i32, w: u32, h: u32) -> &'static str {
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

/// Ms to ADD to an event-clock timestamp to land on the OUTPUT clock, saturating at 0 - the same
/// shape as `edit::migrate::output_shift`'s recipe (this fn takes the already-computed shift
/// rather than a `ProjectPaths`, since `ai::commands::build_plan` derives it once and this module
/// has no path/IO concerns of its own). `pub(crate)` so `narrate::zoom_label` can put a click's
/// timestamp on the SAME output clock its `at_ms` is already on (M1) instead of comparing against
/// the raw event clock.
pub(crate) fn sh(t: u32, shift: i64) -> u32 { (t as i64 + shift).max(0) as u32 }

pub fn serialize(
    log: &EventLog, actions: &[ActionEvent], cursor: &CursorTrack,
    typing: &[u32], dur_ms: u32, shift: i64,
) -> String {
    let (w, h) = (log.screen.w, log.screen.h);
    let mut m: Vec<Moment> = Vec::new();

    for ev in &log.events {
        if ev.kind == EventKind::Down {
            let t = sh(ev.t, shift);
            // H1: `ev.x`/`ev.y` are virtual-DESKTOP coordinates (the raw `WH_MOUSE_LL` hook
            // position), not screen-local - on any monitor placed off-origin (anything but the
            // primary) they're wildly outside `[0, w) x [0, h)`. `to_frame` is the same
            // origin-subtraction every other renderer path (`Cursor`, `fx_state`) already applies
            // before touching a screen-sized box; region()/the printed coordinate must too.
            let p = to_frame(&log.screen, ev.x, ev.y);
            m.push(mo(t, format!("{}s click ({},{}) {}", fmt(t), p.x, p.y, region(p.x, p.y, w, h))));
        }
    }

    for act in actions {
        if let ActionKind::SetLayout(id) = act.kind {
            let t = sh(act.t, shift);
            m.push(mo(t, format!("{}s layout -> {}", fmt(t), format!("{:?}", id).to_lowercase())));
        }
    }

    if !typing.is_empty() {
        let (mut s, mut e) = (sh(typing[0], shift), sh(typing[0], shift));
        for &ms in &typing[1..] {
            let ms = sh(ms, shift);
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
        let t = sh(t, shift);
        if ct == CursorType::IBeam && ib.is_none() { ib = Some(t); }
        else if ct != CursorType::IBeam {
            if let Some(start) = ib.take() {
                let end = if i > 0 { sh(smp[i - 1].0, shift) } else { start };
                m.push(mo(start, format!("{}-{}s text field", fmt(start), fmt(end))));
            }
        }
    }
    if let Some(start) = ib {
        let end = smp.last().map(|s| sh(s.0, shift)).unwrap_or(start);
        m.push(mo(start, format!("{}-{}s text field", fmt(start), fmt(end))));
    }

    let times: Vec<u32> = log.events.iter().map(|e| sh(e.t, shift)).collect();
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
#[path = "timeline_tests.rs"]
mod tests;
