// Pack format v2's animated busy cursor. A v2 `pack.json` may carry `busy: { anim, fps }`; the
// renderer then synthesises the animation from the pack's single `busy.png`. A pack may instead
// ship explicit `busy_00.png .. busy_NN.png` frames, which take precedence.
//
// Everything here is pure math on an OUTPUT-clock timestamp, so the same instant always yields the
// same pose: deterministic per exported frame, and a paused preview shows exactly the frame for
// where the playhead sits. `src/editor/stage/cursorBusy.ts` is the TS mirror, tested against the
// same instants on both sides.
use serde::{Deserialize, Serialize};

/// One 1-second animation cycle. `spin` is the exception: its cycle length is `24 / fps` seconds.
const CYCLE_MS: f64 = 1000.0;
/// `flip` holds this fraction of its cycle, then turns over the rest.
const FLIP_HOLD: f64 = 0.7;
/// `pulse`'s peak scale at mid-cycle.
const PULSE_PEAK: f64 = 1.06;

/// How a pack animates its busy cursor from a single still.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BusyAnim {
    /// One full clockwise turn per cycle - rings and spinners.
    Spin,
    /// Hold, then a 180-degree turn eased over the last 30% - hourglasses.
    Flip,
    /// Breathe 1.0 -> 1.06 -> 1.0 about the hotspot - a sleeping cat, a pocket watch.
    Pulse,
}

/// A pack's busy animation: the declared `anim`/`fps` plus however many explicit `busy_NN.png`
/// frames its folder actually ships (`0` = none, so `anim` drives the pose instead).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BusySpec {
    pub anim: BusyAnim,
    pub fps: f32,
    #[serde(default)]
    pub frames: u32,
}

/// What to draw for the busy cursor at one instant: which frame, and the transform to apply to it
/// about the hotspot. `frame` is always 0 for a synthesised animation; `angle_deg`/`scale` are
/// always the identity for an explicit-frame one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BusyPose {
    pub frame: u32,
    pub angle_deg: f32,
    pub scale: f32,
}

impl BusyPose {
    /// The still, untransformed pose - what a v1 pack (no `busy` block) always draws.
    pub fn still() -> Self {
        Self { frame: 0, angle_deg: 0.0, scale: 1.0 }
    }
    /// Whether this pose needs the transformed blit at all. The identity takes `cursordraw`'s
    /// existing nearest-neighbour fast path, bit-for-bit as before pack v2 existed.
    pub fn is_identity(&self) -> bool {
        self.angle_deg == 0.0 && self.scale == 1.0
    }
}

/// The busy pose at output time `t_ms`. Computed in `f64` so the TS mirror (which has no `f32`)
/// agrees to well under a tenth of a degree.
pub fn busy_pose(spec: &BusySpec, t_ms: u32) -> BusyPose {
    let t = t_ms as f64;
    if spec.frames > 0 {
        let fps = (spec.fps as f64).max(0.001);
        let i = (t / 1000.0 * fps).floor().max(0.0) as u32 % spec.frames;
        return BusyPose { frame: i, ..BusyPose::still() };
    }
    match spec.anim {
        // `fps` sets the cycle: a full turn takes 24/fps seconds, so the default 24 is one turn
        // per second. Clockwise, about the sprite's own hotspot.
        BusyAnim::Spin => BusyPose {
            angle_deg: (360.0 * (t / 1000.0 * spec.fps as f64 / 24.0)).rem_euclid(360.0) as f32,
            ..BusyPose::still()
        },
        BusyAnim::Flip => BusyPose { angle_deg: flip_angle(t) as f32, ..BusyPose::still() },
        BusyAnim::Pulse => BusyPose { scale: pulse_scale(t) as f32, ..BusyPose::still() },
    }
}

/// `flip`: still for the first 70% of the cycle, then a cosine-eased half turn over the last 30%.
/// The completed cycles are ACCUMULATED (`180 * n + ...`) so it always turns the same way instead
/// of snapping back at each cycle boundary - an hourglass tips over, it does not rock.
fn flip_angle(t_ms: f64) -> f64 {
    let cycles = (t_ms / CYCLE_MS).floor();
    let u = t_ms / CYCLE_MS - cycles;
    let turned = if u < FLIP_HOLD { 0.0 } else { 180.0 * ease_in_out((u - FLIP_HOLD) / (1.0 - FLIP_HOLD)) };
    (cycles * 180.0 + turned).rem_euclid(360.0)
}

/// `pulse`: 1.0 at the cycle edges, `PULSE_PEAK` at its middle, on the same cosine family.
fn pulse_scale(t_ms: f64) -> f64 {
    let u = t_ms / CYCLE_MS;
    1.0 + (PULSE_PEAK - 1.0) * bump(u - u.floor())
}

/// Cosine ease 0 -> 1 across `u` in 0..1 (half a cosine period): no jerk at either end.
fn ease_in_out(u: f64) -> f64 {
    (1.0 - (std::f64::consts::PI * u.clamp(0.0, 1.0)).cos()) / 2.0
}

/// Cosine bump: 0 at both ends of `u` in 0..1, 1 at its middle (a full cosine period).
fn bump(u: f64) -> f64 {
    (1.0 - (std::f64::consts::TAU * u.clamp(0.0, 1.0)).cos()) / 2.0
}

#[cfg(test)]
#[path = "busy_tests.rs"]
mod tests;
