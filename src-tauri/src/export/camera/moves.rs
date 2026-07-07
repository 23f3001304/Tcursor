use crate::edit::model::CameraMove;
use crate::export::camera::ease;
use crate::export::render::fromedit::easing_from;
use crate::export::types::Easing;

/// One resolved webcam-PiP position/size at a frame time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose { pub x: f32, pub y: f32, pub size: f32 }

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

    /// `None` for an empty track (caller keeps its static pose). Otherwise: at/before the
    /// first keyframe, ease FROM `static_pose` (the caller's un-overridden static PiP pose)
    /// INTO the first keyframe over `[0, first.t_ms]` using the first keyframe's own easing -
    /// an implicit t=0 keyframe at the static pose, same "ease into the entered segment"
    /// convention `LayoutTrack::scene_at` uses. `static_pose = None` (or `first.t_ms == 0`,
    /// nothing to ease from) holds the first keyframe's pose flat, same as before this
    /// existed. Hold the last keyframe at/after it; between two keyframes `a`/`b` ease INTO
    /// `b` using `b`'s own easing over `[a.t_ms, b.t_ms]` - unchanged.
    pub fn sample(&self, t_ms: u32, static_pose: Option<CamPose>) -> Option<CamPose> {
        if self.kfs.is_empty() { return None; }
        let first = &self.kfs[0];
        if t_ms <= first.t_ms {
            return Some(match static_pose {
                Some(s) if first.t_ms > 0 => {
                    let f = ease(first.easing, t_ms as f32 / first.t_ms as f32);
                    CamPose { x: s.x + (first.x - s.x) * f, y: s.y + (first.y - s.y) * f, size: s.size + (first.size - s.size) * f }
                }
                _ => pose(first),
            });
        }
        let last = self.kfs.len() - 1;
        if t_ms >= self.kfs[last].t_ms { return Some(pose(&self.kfs[last])); }

        // First index whose t_ms is > t_ms; since t_ms is strictly between the first and
        // last keyframe's times (checked above), this always lands in (0, last].
        let bi = self.kfs.iter().position(|k| k.t_ms > t_ms).unwrap();
        let (a, b) = (&self.kfs[bi - 1], &self.kfs[bi]);
        if b.t_ms == a.t_ms { return Some(pose(b)); } // coincident keyframes: no divide-by-zero
        let f = ease(b.easing, (t_ms - a.t_ms) as f32 / (b.t_ms - a.t_ms) as f32);
        Some(CamPose {
            x: a.x + (b.x - a.x) * f,
            y: a.y + (b.y - a.y) * f,
            size: a.size + (b.size - a.size) * f,
        })
    }
}

fn pose(k: &Kf) -> CamPose { CamPose { x: k.x, y: k.y, size: k.size } }

#[cfg(test)]
#[path = "moves_tests.rs"]
mod tests;
