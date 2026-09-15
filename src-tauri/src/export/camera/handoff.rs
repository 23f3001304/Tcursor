use super::ease;
use crate::export::types::{Camera, Easing};

fn h10(u: f32) -> f32 {
    u * (u - 1.0) * (u - 1.0)
}

pub(crate) struct Transition {
    from: Camera,
    vel: (f32, f32, f32),
    t: f32,
    dur: f32,
    easing: Easing,
}

impl Transition {
    pub(crate) fn open(
        from: Camera,
        vel: (f32, f32, f32),
        dt_ms: f32,
        dur_ms: f32,
        easing: Easing,
    ) -> Self {
        Self {
            from,
            vel,
            t: 0.0,
            dur: (dur_ms + dt_ms).max(1.0),
            easing,
        }
    }

    pub(crate) fn blend(&mut self, dt_ms: f32, target: Camera) -> Option<Camera> {
        self.t += dt_ms;
        if self.t >= self.dur {
            return None;
        }
        let u = self.t / self.dur;
        let (e, h) = (ease(self.easing, u), h10(u) * self.dur);
        Some(Camera {
            cx: self.from.cx + (target.cx - self.from.cx) * e + h * self.vel.0,
            cy: self.from.cy + (target.cy - self.from.cy) * e + h * self.vel.1,
            scale: self.from.scale + (target.scale - self.from.scale) * e + h * self.vel.2,
        })
    }
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;
