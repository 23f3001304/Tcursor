// Arrangement edit ops (T34), split out of api.rs so each file stays focused. `api::apply`
// delegates `SetArrangement`/`ClearArrangement` here. Pure doc mutation: both ops ride the one
// serialized load->apply->save the `apply_edit_op` command already performs under `doc_lock`, so
// each is a single atomic doc write (and therefore a single undo step) like every other op.
use crate::edit::model::{Arrangement, EditDoc, PanelPose};
use crate::edit::ops::api::EditOp;
use serde::{Deserialize, Deserializer};

/// Pose bounds. `cx`/`cy` are output-frame fractions, so a panel's CENTER stays on the frame
/// (a panel may still overhang an edge - that is a legitimate bleed-off-frame composition).
/// `size` is the panel's height fraction: below `MIN_SIZE` it is invisible-but-not-hidden (use a
/// `None` panel for hidden), above `MAX_SIZE` it is larger than any frame could show.
const MIN_SIZE: f32 = 0.05;
const MAX_SIZE: f32 = 1.5;

/// Clamp a pose into the addressable range. Applied on the way IN, so the doc never stores a pose
/// that resolution would have to defend against.
pub(crate) fn clamp_pose(p: PanelPose) -> PanelPose {
    PanelPose { cx: p.cx.clamp(0.0, 1.0), cy: p.cy.clamp(0.0, 1.0), size: p.size.clamp(MIN_SIZE, MAX_SIZE) }
}

/// Deserializer for a `Option<Option<T>>` field that must distinguish an ABSENT key ("leave this
/// panel as it is") from an explicit `null` ("hide this panel"). Plain `Option<Option<T>>` cannot:
/// serde folds `null` into the outer `None`, colliding with the `#[serde(default)]` for absent.
/// Deserializing the INNER `Option` and wrapping it keeps the two apart.
pub fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where D: Deserializer<'de>, T: Deserialize<'de> {
    Option::deserialize(d).map(Some)
}

/// Apply one arrangement op. No-op for anything else (the match arm in `api::apply` only routes
/// the two arrangement variants here).
///
/// `SetArrangement` is a partial update over the segment's CURRENT arrangement: a field the caller
/// omitted is left alone, `null` hides that panel, a pose sets (and un-hides) it. On a segment that
/// has no arrangement yet the base is "both panels hidden", so a first conversion from a preset
/// must send BOTH panels - which is exactly what the frontend has, from `arrangement_of_preset`.
/// A change that would hide both panels is REJECTED (the doc is left untouched) rather than saved
/// as an empty frame; `ClearArrangement` is the way back to the preset.
pub fn apply_arrangement(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::SetArrangement { id, screen, cam } => {
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) {
                let cur = s.arrangement.unwrap_or(Arrangement { screen: None, cam: None });
                let pick = |touched: Option<Option<PanelPose>>, now: Option<PanelPose>| match touched {
                    None => now,
                    Some(v) => v.map(clamp_pose),
                };
                let next = Arrangement { screen: pick(screen, cur.screen), cam: pick(cam, cur.cam) };
                if next.screen.is_some() || next.cam.is_some() { s.arrangement = Some(next); }
            }
        }
        EditOp::ClearArrangement { id } => {
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) { s.arrangement = None; }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "arrangement_tests.rs"]
mod tests;
