use crate::actions::model::{ActionEvent, ActionKind};

/// Strength 0..1 at `et`, fading in/out over `fade_ms` around each start..end pair
/// (an unpaired still-held start stays on). Max over pairs. Shared by every
/// hold-to-activate manual effect (spotlight, video FX).
pub fn hold_alpha(
    actions: &[ActionEvent], et: u32, fade_ms: u32,
    is_start: impl Fn(ActionKind) -> bool, is_end: impl Fn(ActionKind) -> bool,
) -> f32 {
    let fade = fade_ms.max(1) as f32;
    let mut alpha = 0.0f32;
    let mut start: Option<u32> = None;
    for a in actions {
        if is_start(a.kind) { start = Some(a.t); }
        else if is_end(a.kind) {
            if let Some(s) = start.take() {
                if et >= s {
                    let up = ((et - s) as f32 / fade).min(1.0);
                    let down = if et >= a.t { (1.0 - (et - a.t) as f32 / fade).max(0.0) } else { 1.0 };
                    alpha = alpha.max(up.min(down));
                }
            }
        }
    }
    if let Some(s) = start { if et >= s { alpha = alpha.max(((et - s) as f32 / fade).min(1.0)); } }
    alpha
}

/// The held intervals as `(start, end)` event-time pairs (ms). An unpaired still-held start
/// runs to `end_default`. Used by the editor preview to surface recorded holds (e.g. spotlight)
/// the same way `click_track` surfaces clicks - the ramp/fade is applied by the consumer.
pub fn hold_spans(
    actions: &[ActionEvent], end_default: u32,
    is_start: impl Fn(ActionKind) -> bool, is_end: impl Fn(ActionKind) -> bool,
) -> Vec<(u32, u32)> {
    let mut spans = Vec::new();
    let mut start: Option<u32> = None;
    for a in actions {
        if is_start(a.kind) { start = Some(a.t); }
        else if is_end(a.kind) { if let Some(s) = start.take() { spans.push((s, a.t.max(s))); } }
    }
    if let Some(s) = start { spans.push((s, end_default.max(s))); }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::model::{ActionEvent, ActionKind};
    fn a(t: u32, k: ActionKind) -> ActionEvent { ActionEvent { t, kind: k } }
    #[test]
    fn ramps_up_holds_then_down() {
        let acts = vec![a(1000, ActionKind::SpotlightHoldStart), a(2000, ActionKind::SpotlightHoldEnd)];
        let s = |k| matches!(k, ActionKind::SpotlightHoldStart);
        let e = |k| matches!(k, ActionKind::SpotlightHoldEnd);
        assert_eq!(hold_alpha(&acts, 900, 200, s, e), 0.0);
        assert!((hold_alpha(&acts, 1100, 200, s, e) - 0.5).abs() < 0.01);
        assert_eq!(hold_alpha(&acts, 1500, 200, s, e), 1.0);
        assert!((hold_alpha(&acts, 2100, 200, s, e) - 0.5).abs() < 0.01);
        assert_eq!(hold_alpha(&acts, 2300, 200, s, e), 0.0);
    }
    #[test]
    fn spans_pair_and_unpaired_runs_to_default() {
        let acts = vec![a(1000, ActionKind::SpotlightHoldStart), a(2000, ActionKind::SpotlightHoldEnd),
            a(3000, ActionKind::SpotlightHoldStart)];
        let s = |k| matches!(k, ActionKind::SpotlightHoldStart);
        let e = |k| matches!(k, ActionKind::SpotlightHoldEnd);
        assert_eq!(hold_spans(&acts, 5000, s, e), vec![(1000, 2000), (3000, 5000)]);
    }
}
