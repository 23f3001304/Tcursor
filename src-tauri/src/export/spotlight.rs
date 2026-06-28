use crate::actions::model::{ActionEvent, ActionKind};

/// Spotlight strength 0..1 at `et`. Delegates to the generic hold ramp.
pub fn hold_alpha(actions: &[ActionEvent], et: u32, fade_ms: u32) -> f32 {
    crate::export::hold::hold_alpha(actions, et, fade_ms,
        |k| matches!(k, ActionKind::SpotlightHoldStart),
        |k| matches!(k, ActionKind::SpotlightHoldEnd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::model::ActionEvent;
    fn a(t: u32, k: ActionKind) -> ActionEvent { ActionEvent { t, kind: k } }
    #[test]
    fn ramps_up_holds_then_ramps_down() {
        let acts = vec![a(1000, ActionKind::SpotlightHoldStart), a(2000, ActionKind::SpotlightHoldEnd)];
        assert_eq!(hold_alpha(&acts, 900, 200), 0.0);          // before
        assert!((hold_alpha(&acts, 1100, 200) - 0.5).abs() < 0.01); // 100ms into 200ms ramp-up
        assert_eq!(hold_alpha(&acts, 1500, 200), 1.0);          // held
        assert!((hold_alpha(&acts, 2100, 200) - 0.5).abs() < 0.01); // 100ms into ramp-down
        assert_eq!(hold_alpha(&acts, 2300, 200), 0.0);          // fully out
    }
    #[test]
    fn unpaired_start_stays_on() {
        let acts = vec![a(1000, ActionKind::SpotlightHoldStart)];
        assert_eq!(hold_alpha(&acts, 5000, 200), 1.0);
    }
}
