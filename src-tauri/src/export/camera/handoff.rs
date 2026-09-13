// One in-flight handoff between whichever region was "in control" and the new winner. It eases
// the camera from wherever it actually is toward the new winner's own natural target over
// `dur`, using the entering (or exiting) region's easing curve - and CARRIES the camera's
// velocity across the seam, which a plain position ease cannot (`ease'(0) = 0` for smoothstep,
// so a moving camera was stopped dead and re-launched). Its clock is the caller's EXACT step
// length in ms, accumulated here, not a difference of rounded frame timestamps. See `CameraSim::step`.
use super::ease;
use crate::export::types::{Camera, Easing};

/// Cubic-Hermite's velocity basis `u^3 - 2u^2 + u`, factored. It is 0 at both ends (so the blend
/// still starts at the captured pose and still lands exactly on the target), has slope 1 at u=0
/// (so scaling it by `dur * v0` reproduces the camera's real entry velocity) and slope 0 at
/// u=1 (so the blend arrives with the target's own velocity, not a leftover of the old one).
fn h10(u: f32) -> f32 { u * (u - 1.0) * (u - 1.0) }

pub(crate) struct Transition {
    from: Camera,
    /// `(cx, cy, scale)` velocity at the handoff, per MILLISECOND (not per step).
    vel: (f32, f32, f32),
    t: f32, dur: f32, easing: Easing,
}

impl Transition {
    /// `dt_ms` is the step the handoff was detected on. The blend's own clock starts at 0 and is
    /// advanced by `blend`, so its FIRST sample lands one step in - the seam is the PREVIOUS
    /// sample, whose pose `from` already is. `dur` is stretched by that same step so the blend
    /// still ends exactly where the incoming region's ramp does.
    pub(crate) fn open(from: Camera, vel: (f32, f32, f32), dt_ms: f32, dur_ms: f32, easing: Easing) -> Self {
        Self { from, vel, t: 0.0, dur: (dur_ms + dt_ms).max(1.0), easing }
    }

    /// Advance by `dt_ms` and return the blended pose on the way to `target`, or `None` once the
    /// blend has run its course - at which point the caller drops it and takes `target` exactly.
    pub(crate) fn blend(&mut self, dt_ms: f32, target: Camera) -> Option<Camera> {
        self.t += dt_ms;
        if self.t >= self.dur { return None; }
        let u = self.t / self.dur;
        let (e, h) = (ease(self.easing, u), h10(u) * self.dur);
        Some(Camera {
            cx: self.from.cx + (target.cx - self.from.cx) * e + h * self.vel.0,
            cy: self.from.cy + (target.cy - self.from.cy) * e + h * self.vel.1,
            scale: self.from.scale + (target.scale - self.from.scale) * e + h * self.vel.2,
        })
    }
}

#[cfg(test)] #[path = "handoff_tests.rs"] mod tests;
