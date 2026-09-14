use crate::edit::model::CameraMove;
use crate::export::camera::ease;
use crate::export::render::fromedit::easing_from;
use crate::export::types::Easing;

/// One resolved webcam-PiP position/size/shape at a frame time. `round` is the corner radius as
/// a fraction of the panel's SHORT side (0 = rect, 0.5 = circle) - `None` means "whatever the
/// static panel's radius is, scaled with the resize" (an arrangement pose, or a keyframe that
/// inherits the layout's shape and was sampled without a live pose to inherit from).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32, pub round: Option<f32> }

/// Handoff length (ms) on EACH side of the keyframe span, between the live layout-resolved
/// pose and the track. A module constant, deliberately not a setting.
pub const KF_BLEND_MS: u32 = 350;

/// One sorted keyframe, its wire-easing pre-resolved to `Easing` and its wire-shape to a corner
/// fraction (`None` = inherit the layout's) once at construction.
struct Kf { t_ms: u32, x: f32, y: f32, size: f32, round: Option<f32>, easing: Easing }

/// A keyframe's `shape`/`roundness` as the corner fraction `CamPose::round` carries.
pub fn shape_round(shape: &str, roundness: f32) -> Option<f32> {
    match shape {
        "circle" => Some(0.5),
        "rect" => Some(0.0),
        "rounded" => Some(roundness.clamp(0.0, 0.5)),
        _ => None,
    }
}

/// Keyframed webcam PiP position+size(+shape) track (`EditDoc.camera_moves`). Empty means
/// "no override" - the caller keeps whatever static `overlay_for` rect it already has,
/// so a doc with no camera_moves renders byte-identically to today. Mirrors
/// `LayoutTrack`'s "ease into the entered keyframe using its own easing" convention.
pub struct CameraMoveTrack { kfs: Vec<Kf> }

impl CameraMoveTrack {
    /// Clones + sorts by `t_ms` (defensive - callers should already hand these in order,
    /// e.g. straight off `EditDoc.camera_moves`, but this must never trust that).
    pub fn from_doc(moves: &[CameraMove]) -> Self {
        let mut kfs: Vec<Kf> = moves.iter().map(|m| Kf {
            t_ms: m.t_ms, x: m.x, y: m.y, size: m.size, round: shape_round(&m.shape, m.roundness),
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
    /// ("live") pose for THIS frame, re-read every call, not a one-off static pose; a keyframe
    /// whose shape is "layout" takes its `round` from it.
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
        let pose = |k: &Kf| CamPose { x: k.x, y: k.y, size: k.size, round: k.round.or(live.and_then(|l| l.round)) };
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

/// Component-wise lerp of a whole pose (x/y/size/round); the rect is derived from the result
/// once, by `rect_from_center`, so the T14 aspect handling applies to the blended pose too. A
/// `round` only one side knows is carried through unblended; two unknowns stay unknown.
fn mix(a: CamPose, b: CamPose, f: f32) -> CamPose {
    let round = match (a.round, b.round) {
        (Some(ra), Some(rb)) => Some(ra + (rb - ra) * f),
        (ra, rb) => ra.or(rb),
    };
    CamPose { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f, round }
}

#[cfg(test)]
#[path = "moves_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "moves_span_tests.rs"]
mod span_tests;
