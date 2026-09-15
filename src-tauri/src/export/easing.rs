use crate::export::types::Easing;

pub fn ease(e: Easing, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match e {
        Easing::Linear => t,
        Easing::Smooth => 1.0 - (1.0 - t).powi(3),
        Easing::Spring { .. } => crate::export::spring::eval(e, t),
        Easing::EaseIn => t * t,
        Easing::EaseOut => t * (2.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - 2.0 * (1.0 - t) * (1.0 - t)
            }
        }
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::eval(x1, y1, x2, y2, t),
        Easing::Keys(ref k) => crate::export::keys::eval(k, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smooth_hits_endpoints_and_eases_out() {
        assert!((ease(Easing::Smooth, 0.0) - 0.0).abs() < 1e-6);
        assert!((ease(Easing::Smooth, 1.0) - 1.0).abs() < 1e-6);
        assert!(ease(Easing::Smooth, 0.5) > 0.5);
    }
    #[test]
    fn linear_is_identity_clamped() {
        assert_eq!(ease(Easing::Linear, 0.3), 0.3);
        assert_eq!(ease(Easing::Linear, -1.0), 0.0);
        assert_eq!(ease(Easing::Linear, 2.0), 1.0);
    }
    #[test]
    fn ease_in_out_curves_hit_endpoints_and_bend() {
        for e in [Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut] {
            assert!(ease(e, 0.0).abs() < 1e-6);
            assert!((ease(e, 1.0) - 1.0).abs() < 1e-6);
        }
        assert!(ease(Easing::EaseIn, 0.5) < 0.5);
        assert!(ease(Easing::EaseOut, 0.5) > 0.5);
        assert!((ease(Easing::EaseInOut, 0.5) - 0.5).abs() < 1e-6);
    }
}
