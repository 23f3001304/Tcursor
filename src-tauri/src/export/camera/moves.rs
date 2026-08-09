use crate::edit::model::CameraMove;
use crate::export::camera::ease;
use crate::export::render::fromedit::easing_from;
use crate::export::types::Easing;

/// One resolved webcam-PiP position/size at a frame time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32 }

/// Handoff length (ms) on EACH side of the keyframe span, between the live layout-resolved
/// pose and the track. A module constant, deliberately not a setting.
pub const KF_BLEND_MS: u32 = 350;

/// One sorted keyframe, its wire-easing pre-resolved to `Easing` once at construction.
struct Kf { t_ms: u32, x: f32, y: f32, size: f32, easing: Easing }

/// Keyframed webcam PiP position+size track (`EditDoc.camera_moves`). Empty means
/// "no override" - the caller keeps whatever static `overlay_for` rect it already has,
/// so a doc with no camera_moves renders byte-identically to today. Mirrors
/// `LayoutTrack`'s "ease into the entered keyframe using its own easing" convention.
pub struct CameraMoveTrack { kfs: Vec<Kf> }

impl CameraMoveTrack {
    /// Clones + sorts by `t_ms` (defensive - callers should already hand these in order,
    /// e.g. straight off `EditDoc.camera_moves`, but this must never trust that).
    pub fn from_doc(moves: &[CameraMove]) -> Self {
        let mut kfs: Vec<Kf> = moves.iter().map(|m| Kf {
            t_ms: m.t_ms, x: m.x, y: m.y, size: m.size,
            easing: easing_from(&m.easing, Easing::Smooth),
        }).collect();
        kfs.sort_by_key(|k| k.t_ms);
        Self { kfs }
    }

    /// The half-open-ish span this track OWNS: `[first - KF_BLEND_MS, last + KF_BLEND_MS]`
    /// (saturating at 0). `None` for an empty track. Outside it `sample` is `None`.
    pub fn span(&self) -> Option<(u32, u32)> {
        Some((self.kfs.first()?.t_ms.saturating_sub(KF_BLEND_MS),
              self.kfs.last()?.t_ms.saturating_add(KF_BLEND_MS)))
    }

    /// The PiP pose at `t_ms`, or `None` when the keyframes do NOT own this frame - the
    /// caller then leaves the layout-resolved panel alone. `live` is that layout-resolved
    /// ("live") pose for THIS frame, re-read every call, not a one-off static pose.
    ///
    /// Five cases: outside `span()` -> `None` (layout owns it); `[first - BLEND, first)` ->
    /// ease FROM `live` INTO the first keyframe with the first keyframe's own easing;
    /// `[first, last]` -> keyframe interpolation, easing INTO `b` with `b`'s own easing
    /// (unchanged math); `(last, last + BLEND]` -> ease FROM the last keyframe BACK to
    /// `live`, which tracks a moving target because it is re-evaluated per frame. A single
    /// keyframe therefore eases in, hits its pose for that instant, and eases back out (a
    /// hold needs two keyframes). `live = None` skips both blends and snaps to the nearest
    /// end keyframe; the span rule itself never depends on it.
    pub fn sample(&self, t_ms: u32, live: Option<CamPose>) -> Option<CamPose> {
        let (entry, exit) = self.span()?;
        let (first, last) = (self.kfs.first()?, self.kfs.last()?);
        if t_ms < entry || t_ms > exit { return None; }
        if t_ms < first.t_ms {
            let win = first.t_ms - entry; // == KF_BLEND_MS unless clamped at t=0
            return Some(match live {
                Some(l) if win > 0 => mix(l, pose(first), ease(first.easing, (t_ms - entry) as f32 / win as f32)),
                _ => pose(first),
            });
        }
        if t_ms > last.t_ms {
            return Some(match live {
                Some(l) => mix(pose(last), l, ease(last.easing, (t_ms - last.t_ms) as f32 / KF_BLEND_MS as f32)),
                None => pose(last),
            });
        }
        if t_ms == last.t_ms { return Some(pose(last)); } // also the single-keyframe instant

        // First index whose t_ms is > t_ms; since t_ms is inside [first, last) here, this
        // always lands in (0, last].
        let bi = self.kfs.iter().position(|k| k.t_ms > t_ms).unwrap();
        let (a, b) = (&self.kfs[bi - 1], &self.kfs[bi]);
        if b.t_ms == a.t_ms { return Some(pose(b)); } // coincident keyframes: no divide-by-zero
        let f = ease(b.easing, (t_ms - a.t_ms) as f32 / (b.t_ms - a.t_ms) as f32);
        Some(mix(pose(a), pose(b), f))
    }
}

fn pose(k: &Kf) -> CamPose { CamPose { x: k.x, y: k.y, size: k.size } }

/// Component-wise lerp of a whole pose (x/y/size); the rect is derived from the result once,
/// by `rect_from_center`, so the T14 aspect handling applies to the blended pose too.
fn mix(a: CamPose, b: CamPose, f: f32) -> CamPose {
    CamPose { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f }
}

#[cfg(test)]
#[path = "moves_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "moves_span_tests.rs"]
mod span_tests;
