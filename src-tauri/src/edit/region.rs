// Small shared helpers for placing/bounding timeline regions (zooms and effects alike),
// split out of api.rs so that file stays under the size limit.
use crate::edit::model::EditDoc;

/// The document's known upper time bound, or `u32::MAX` if the trim hasn't been set yet (should
/// not happen in practice - `edit::seed` always seeds `trim.out_ms` to the real clip duration -
/// but this keeps a not-yet-seeded doc from collapsing every region to zero length).
pub(crate) fn dur_bound(doc: &EditDoc) -> u32 { if doc.trim.out_ms > 0 { doc.trim.out_ms } else { u32::MAX } }

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

#[cfg(test)]
mod tests {
    use super::*;

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
