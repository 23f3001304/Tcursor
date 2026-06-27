use crate::events::model::{EventKind, MouseEvent};

/// A live click effect at output time: the click's screen point and its progress
/// 0..1 through the effect lifetime. The exporter maps `(sx,sy)` through the active
/// scene + zoom; `progress` drives radius and alpha. Newest hit is last.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit { pub sx: i32, pub sy: i32, pub progress: f32 }

/// Click effects alive at event-time `et`: each `Down` within the last `life_ms`
/// contributes a `Hit`.
pub fn hits_at(events: &[MouseEvent], et: u32, life_ms: u32) -> Vec<Hit> {
    let life = life_ms.max(1) as f32;
    events.iter()
        .filter(|e| matches!(e.kind, EventKind::Down) && et >= e.t && et - e.t < life_ms)
        .map(|e| Hit { sx: e.x, sy: e.y, progress: (et - e.t) as f32 / life })
        .collect()
}

/// Ripple ring radius (output px) at `progress`, growing to `r_max`.
pub fn ripple_radius(progress: f32, r_max: f32) -> f32 { progress.clamp(0.0, 1.0) * r_max }

/// Effect opacity at `progress` (fades out), scaled by the user intensity.
pub fn fade_alpha(progress: f32, intensity: f32) -> f32 {
    (1.0 - progress).clamp(0.0, 1.0) * intensity.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind, MouseEvent};
    fn down(t: u32, x: i32, y: i32) -> MouseEvent { MouseEvent { t, kind: EventKind::Down, x, y, button: Some(Button::Left) } }
    fn mv(t: u32) -> MouseEvent { MouseEvent { t, kind: EventKind::Move, x: 0, y: 0, button: None } }

    #[test]
    fn only_downs_within_lifetime_are_active() {
        let ev = vec![down(0, 10, 20), mv(50), down(100, 30, 40)];
        // at t=300, life=600: both downs alive (0 and 100); the move is ignored.
        let h = hits_at(&ev, 300, 600);
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].sx, h[0].sy), (10, 20));
        // the older click has the larger progress.
        assert!(h[0].progress > h[1].progress);
    }

    #[test]
    fn expired_and_future_clicks_are_excluded() {
        let ev = vec![down(0, 1, 1), down(1000, 2, 2)];
        let h = hits_at(&ev, 700, 600); // first expired (700-0>=600), second not yet (700<1000)
        assert!(h.is_empty());
    }

    #[test]
    fn radius_grows_and_alpha_fades() {
        assert!((ripple_radius(0.0, 100.0)).abs() < 1e-6);
        assert!((ripple_radius(1.0, 100.0) - 100.0).abs() < 1e-6);
        assert!(fade_alpha(0.0, 1.0) > fade_alpha(0.9, 1.0));
        assert!((fade_alpha(0.5, 0.5) - 0.25).abs() < 1e-6); // (1-0.5)*0.5
    }
}
