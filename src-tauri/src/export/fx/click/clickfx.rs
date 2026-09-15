use crate::events::model::{EventKind, MouseEvent};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub sx: i32,
    pub sy: i32,
    pub progress: f32,
}

pub fn hits_at(events: &[MouseEvent], et: u32, life_ms: u32) -> Vec<Hit> {
    let life = life_ms.max(1) as f32;
    events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Down) && et >= e.t && et - e.t < life_ms)
        .map(|e| Hit {
            sx: e.x,
            sy: e.y,
            progress: (et - e.t) as f32 / life,
        })
        .collect()
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn ease_out(progress: f32) -> f32 {
    let q = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - q * q * q
}

pub fn fade_alpha(progress: f32, intensity: f32) -> f32 {
    (1.0 - smoothstep(0.55, 1.0, progress.clamp(0.0, 1.0))) * intensity.clamp(0.0, 1.0)
}

pub fn ripple_radius(progress: f32, r_max: f32) -> f32 {
    ease_out(progress) * r_max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind, MouseEvent};
    fn down(t: u32, x: i32, y: i32) -> MouseEvent {
        MouseEvent {
            t,
            kind: EventKind::Down,
            x,
            y,
            button: Some(Button::Left),
        }
    }
    fn mv(t: u32) -> MouseEvent {
        MouseEvent {
            t,
            kind: EventKind::Move,
            x: 0,
            y: 0,
            button: None,
        }
    }

    #[test]
    fn only_downs_within_lifetime_are_active() {
        let ev = vec![down(0, 10, 20), mv(50), down(100, 30, 40)];
        let h = hits_at(&ev, 300, 600);
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].sx, h[0].sy), (10, 20));
        assert!(h[0].progress > h[1].progress);
    }

    #[test]
    fn expired_and_future_clicks_are_excluded() {
        let ev = vec![down(0, 1, 1), down(1000, 2, 2)];
        let h = hits_at(&ev, 700, 600);
        assert!(h.is_empty());
    }

    #[test]
    fn ease_out_is_pinned_at_five_points() {
        for (p, want) in [
            (0.0, 0.0),
            (0.25, 0.578125),
            (0.5, 0.875),
            (0.75, 0.984375),
            (1.0, 1.0),
        ] {
            assert!(
                (ease_out(p) - want).abs() < 1e-6,
                "ease_out({p}) = {} want {want}",
                ease_out(p)
            );
        }
    }

    #[test]
    fn ease_out_clamps_outside_the_life() {
        assert_eq!(ease_out(-1.0), 0.0);
        assert_eq!(ease_out(2.0), 1.0);
    }

    #[test]
    fn fade_alpha_is_pinned_at_five_points() {
        for (p, want) in [
            (0.0, 1.0),
            (0.25, 1.0),
            (0.5, 1.0),
            (0.75, 0.582990_4),
            (1.0, 0.0),
        ] {
            let got = fade_alpha(p, 1.0);
            assert!(
                (got - want).abs() < 1e-5,
                "fade_alpha({p}) = {got} want {want}"
            );
        }
    }

    #[test]
    fn fade_alpha_scales_by_intensity_and_clamps() {
        assert!((fade_alpha(0.5, 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(fade_alpha(-1.0, 2.0), 1.0);
        assert_eq!(fade_alpha(2.0, 1.0), 0.0);
    }

    #[test]
    fn radius_eases_out_to_the_max() {
        assert!((ripple_radius(0.0, 100.0)).abs() < 1e-6);
        assert!((ripple_radius(1.0, 100.0) - 100.0).abs() < 1e-6);
        assert!((ripple_radius(0.5, 100.0) - 87.5).abs() < 1e-4);
    }
}
