use crate::edit::model::EditDoc;

pub(crate) fn dur_bound(doc: &EditDoc) -> u32 {
    if doc.clip_ms > 0 {
        doc.clip_ms
    } else if doc.trim.out_ms > 0 {
        doc.trim.out_ms
    } else {
        u32::MAX
    }
}

pub(crate) fn auto_layer(existing: &[(u32, u32, u32)], start_ms: u32, end_ms: u32) -> u32 {
    let mut layer = 0u32;
    loop {
        let collides = existing
            .iter()
            .any(|&(s, e, l)| l == layer && s < end_ms && e > start_ms);
        if !collides {
            return layer;
        }
        layer += 1;
    }
}

pub(crate) fn clamp_order(start: &mut u32, end: &mut u32, start_was_set: bool) {
    if *start > *end {
        if start_was_set {
            *end = *start;
        } else {
            *start = *end;
        }
    }
}

pub(crate) const CAM_KF_SNAP_MS: u32 = 60;

pub(crate) fn valid_cam_shape(s: &str) -> String {
    match s {
        "layout" | "circle" | "rounded" | "rect" => s.to_string(),
        _ => "layout".into(),
    }
}

pub(crate) fn valid_layout(s: &str) -> String {
    match s {
        "screen" | "camera" | "presenter" | "screen_only" | "camera_only" => s.to_string(),
        _ => "screen".into(),
    }
}

pub(crate) fn valid_easing(s: &str) -> String {
    match s {
        "linear" | "smooth" | "spring" | "ease_in" | "ease_out" | "ease_in_out" => s.to_string(),
        _ => crate::export::spring::parse_spring(s)
            .map(|(k, c, m)| crate::export::spring::format_spring(k, c, m))
            .or_else(|| {
                crate::export::cubic::parse_cubic(s)
                    .map(|(x1, y1, x2, y2)| crate::export::cubic::format_cubic(x1, y1, x2, y2))
            })
            .or_else(|| {
                crate::export::keys::parse_keys(s).map(|k| crate::export::keys::format_keys(&k))
            })
            .unwrap_or_else(|| "smooth".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_easing_keeps_named_curves_and_canonicalises_cubics() {
        assert_eq!(valid_easing("ease_in_out"), "ease_in_out");
        assert_eq!(
            valid_easing("cubic(0.25,0.1,0.25,1)"),
            "cubic(0.250,0.100,0.250,1.000)"
        );
        assert_eq!(
            valid_easing("cubic(-1,0.5,2,0.5)"),
            "cubic(0.000,0.500,1.000,0.500)"
        );
        assert_eq!(valid_easing("cubic(1,2)"), "smooth");
        assert_eq!(valid_easing("wobble"), "smooth");
        assert_eq!(valid_easing("spring"), "spring");
        assert_eq!(
            valid_easing("spring(300,10)"),
            "spring(300.000,10.000,1.000)"
        );
        assert_eq!(
            valid_easing("spring(99999,-4,50)"),
            "spring(2000.000,0.000,10.000)"
        );
        assert_eq!(valid_easing("spring(170)"), "smooth");
    }

    #[test]
    fn valid_easing_canonicalises_keyframed_curves() {
        assert_eq!(
            valid_easing("keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)"),
            "keys(0.000 0.000 0.000 0.000 0.333 0.000 b,1.000 1.000 -0.333 0.000 0.000 0.000 b)"
        );
        assert_eq!(
            valid_easing("keys(1 1 -0.4 0 0 0 b,0 0 0 0 0.1 0.7 b)"),
            "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)"
        );
        assert_eq!(
            valid_easing(
                "keys(0 0 0 0 0 0 l,0.125 0.1 0 0 0 0 l,0.25 0.2 0 0 0 0 l,\
            0.375 0.3 0 0 0 0 l,0.5 0.4 0 0 0 0 l,0.625 0.5 0 0 0 0 l,0.75 0.6 0 0 0 0 l,\
            0.875 0.8 0 0 0 0 l,1 1 0 0 0 0 l)"
            ),
            "smooth"
        );
        assert_eq!(
            valid_easing("keys(0.1 0 0 0 0 0 l,1 1 0 0 0 0 l)"),
            "smooth"
        );
        assert_eq!(
            valid_easing("keys(0 0 0 0 0 0 l,0.9 1 0 0 0 0 l)"),
            "smooth"
        );
        assert_eq!(valid_easing("keys(0 0 0 0 0 0 z,1 1 0 0 0 0 l)"), "smooth");
    }

    #[test]
    fn auto_layer_finds_lowest_free_layer() {
        let existing = vec![(0, 1000, 0), (500, 1500, 1)];
        assert_eq!(auto_layer(&existing, 200, 600), 2);
    }

    #[test]
    fn auto_layer_reuses_a_free_layer_that_does_not_overlap() {
        let existing = vec![(0, 1000, 0)];
        assert_eq!(auto_layer(&existing, 2000, 3000), 0);
    }

    #[test]
    fn clamp_order_pulls_end_to_a_start_dragged_past_it() {
        let (mut s, mut e) = (8000u32, 5000u32);
        clamp_order(&mut s, &mut e, true);
        assert_eq!((s, e), (8000, 8000));
    }

    #[test]
    fn clamp_order_pulls_start_to_an_end_dragged_before_it() {
        let (mut s, mut e) = (5000u32, 1000u32);
        clamp_order(&mut s, &mut e, false);
        assert_eq!((s, e), (1000, 1000));
    }

    #[test]
    fn clamp_order_is_a_noop_when_already_ordered() {
        let (mut s, mut e) = (100u32, 200u32);
        clamp_order(&mut s, &mut e, true);
        assert_eq!((s, e), (100, 200));
    }
}
