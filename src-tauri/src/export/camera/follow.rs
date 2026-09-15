use crate::export::types::{FramePoint, ZoomRegion};

pub(crate) const REF_STEP_MS: f32 = 1000.0 / 60.0;

pub(crate) fn damping(k_ref: f32, dt_ms: f32) -> f32 {
    let k = k_ref.clamp(0.0, 1.0);
    if k >= 1.0 || k <= 0.0 {
        return k;
    }
    let dt = if dt_ms.is_finite() {
        dt_ms.clamp(0.0, 1000.0)
    } else {
        REF_STEP_MS
    };
    1.0 - (1.0 - k).powf(dt / REF_STEP_MS)
}

pub(crate) fn aim(r: &ZoomRegion, cursor: FramePoint) -> (f32, f32) {
    if r.follow_cursor {
        (cursor.x as f32, cursor.y as f32)
    } else {
        (r.anchor.x as f32, r.anchor.y as f32)
    }
}

#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
