// Where a zoom region wants the camera centred, and how fast the centre may close on it. One
// definition, read by EVERY phase of `CameraSim::step`, which is what makes the phase boundaries
// continuous: the zoom-in ramp eases toward exactly the point the hold phase then damps toward,
// so the aim cannot change - and the centre cannot reverse - at the boundary between them.
use crate::export::types::{FramePoint, ZoomRegion};


/// The step length `ZoomConfig::follow_damping` was tuned at: one frame of 60fps output. At
/// exactly this `dt` the effective factor IS the configured one, so the shipped feel is unchanged.
pub(crate) const REF_STEP_MS: f32 = 1000.0 / 60.0;

/// The first-order lerp factor for a step of `dt_ms`, from the per-60fps-frame setting `k_ref`.
/// Shared by the camera's hold/zoom-out follow and the cursor low-pass (`export/cursor/mod.rs`),
/// so one setting means one time constant no matter which filter or which output rate reads it.
pub(crate) fn damping(k_ref: f32, dt_ms: f32) -> f32 {
    let k = k_ref.clamp(0.0, 1.0);
    if k >= 1.0 || k <= 0.0 { return k; }
    let dt = if dt_ms.is_finite() { dt_ms.clamp(0.0, 1000.0) } else { REF_STEP_MS };
    1.0 - (1.0 - k).powf(dt / REF_STEP_MS)
}

/// Region `r`'s aim point for a cursor at `cursor`, in output pixels. A follow region aims at the
/// live cursor; an ANCHORED region aims at its anchor and nothing else. (It used to slide toward
/// the cursor once the cursor left a dead zone around the anchor, "to keep the cursor in view";
/// on a zoom the user aimed at a region that read as the zoom following the cursor anyway, which
/// is exactly what choosing Region says it will not do. Owner ruling, 2026-09-13.)
pub(crate) fn aim(r: &ZoomRegion, cursor: FramePoint) -> (f32, f32) {
    if r.follow_cursor { (cursor.x as f32, cursor.y as f32) } else { (r.anchor.x as f32, r.anchor.y as f32) }
}

#[cfg(test)] #[path = "follow_tests.rs"] mod tests;
