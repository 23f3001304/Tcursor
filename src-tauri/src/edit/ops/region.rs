// Small shared helpers for placing/bounding timeline regions (zooms and effects alike),
// split out of api.rs so that file stays under the size limit.
use crate::edit::model::EditDoc;

/// The document's known upper time bound for PLACING a new/moved region. `clip_ms` (the true
/// recording length) wins when known, so trimming the clip does not collapse a region added past
/// the trim point; `trim.out_ms` is the fallback for a doc predating `clip_ms`; `u32::MAX` is the
/// last resort for a not-yet-seeded doc (should not happen in practice - `edit::seed` always seeds
/// both fields to the real clip duration).
pub(crate) fn dur_bound(doc: &EditDoc) -> u32 {
    if doc.clip_ms > 0 { doc.clip_ms } else if doc.trim.out_ms > 0 { doc.trim.out_ms } else { u32::MAX }
}

/// Assigns a new region to the lowest layer (0, 1, 2, ...) with no existing region
/// overlapping `[start_ms, end_ms)` on that layer. Existing regions' own layers are read
/// as-is and never reassigned - this only decides where a NEW region lands, so it never
/// disturbs a layer the user already set manually. `existing` is `(start_ms, end_ms, layer)`
/// per region - zoom and effect regions both flow through this same rule.
pub(crate) fn auto_layer(existing: &[(u32, u32, u32)], start_ms: u32, end_ms: u32) -> u32 {
    let mut layer = 0u32;
    loop {
        let collides = existing.iter().any(|&(s, e, l)| l == layer && s < end_ms && e > start_ms);
        if !collides { return layer; }
        layer += 1;
    }
}

/// Known layout preset wire-names; anything else falls back to "screen".
pub(crate) fn valid_layout(s: &str) -> String {
    match s { "screen" | "camera" | "presenter" | "screen_only" | "camera_only" => s.to_string(), _ => "screen".into() }
}

/// Coerce an easing wire-name to something `easing_from` can reconstruct: one of the six named
/// curves, or a well-formed custom `cubic(x1,y1,x2,y2)` re-emitted in canonical form (which also
/// applies the x-clamp). Anything else degrades to "smooth" rather than being stored as garbage.
pub(crate) fn valid_easing(s: &str) -> String {
    match s {
        "linear" | "smooth" | "spring" | "ease_in" | "ease_out" | "ease_in_out" => s.to_string(),
        _ => crate::export::cubic::parse_cubic(s)
            .map(|(x1, y1, x2, y2)| crate::export::cubic::format_cubic(x1, y1, x2, y2))
            .unwrap_or_else(|| "smooth".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_easing_keeps_named_curves_and_canonicalises_cubics() {
        assert_eq!(valid_easing("ease_in_out"), "ease_in_out");
        assert_eq!(valid_easing("cubic(0.25,0.1,0.25,1)"), "cubic(0.250,0.100,0.250,1.000)");
        // Out-of-range x is clamped rather than rejected, so a slightly-off client value survives.
        assert_eq!(valid_easing("cubic(-1,0.5,2,0.5)"), "cubic(0.000,0.500,1.000,0.500)");
        // Garbage still degrades to the tuned default.
        assert_eq!(valid_easing("cubic(1,2)"), "smooth");
        assert_eq!(valid_easing("wobble"), "smooth");
    }

    #[test]
    fn auto_layer_finds_lowest_free_layer() {
        // Layer 0 has [0,1000); layer 1 has [500,1500). A new region at [200,600) collides
        // with layer 0's [0,1000) and layer 1's [500,1500) - it must land on layer 2.
        let existing = vec![(0, 1000, 0), (500, 1500, 1)];
        assert_eq!(auto_layer(&existing, 200, 600), 2);
    }

    #[test]
    fn auto_layer_reuses_a_free_layer_that_does_not_overlap() {
        // Layer 0 has [0,1000). A new region at [2000,3000) doesn't overlap it - reuse layer 0.
        let existing = vec![(0, 1000, 0)];
        assert_eq!(auto_layer(&existing, 2000, 3000), 0);
    }
}
