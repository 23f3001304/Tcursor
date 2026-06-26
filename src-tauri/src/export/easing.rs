use crate::export::types::Easing;

/// Map t in [0,1] to an eased value in [0,1] (input clamped).
pub fn ease(e: Easing, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match e {
        Easing::Linear => t,
        Easing::Smooth => 1.0 - (1.0 - t).powi(3), // ease-out cubic
        Easing::Spring { .. } => 1.0 - (1.0 - t).powi(3), // M2b: behave as Smooth; real spring is M3+
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smooth_hits_endpoints_and_eases_out() {
        assert!((ease(Easing::Smooth, 0.0) - 0.0).abs() < 1e-6);
        assert!((ease(Easing::Smooth, 1.0) - 1.0).abs() < 1e-6);
        assert!(ease(Easing::Smooth, 0.5) > 0.5); // ease-out is past halfway at t=0.5
    }
    #[test]
    fn linear_is_identity_clamped() {
        assert_eq!(ease(Easing::Linear, 0.3), 0.3);
        assert_eq!(ease(Easing::Linear, -1.0), 0.0);
        assert_eq!(ease(Easing::Linear, 2.0), 1.0);
    }
}
