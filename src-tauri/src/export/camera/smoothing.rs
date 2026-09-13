// Opt-in post-pass for `CameraSim::step`: a critically damped second-order filter on the pose it
// just produced. Causal, per-step, one knob (`ZoomConfig::smoothing_ms`); 0 is off and returns the
// camera untouched, so the shipped trajectory is bit-identical unless a caller asks for it.
use crate::export::types::Camera;

/// `w * smoothing_ms` at which a critically damped step response `1 - (1 + w t) e^(-w t)` has
/// covered ~96% of the step - i.e. `smoothing_ms` reads as "time to settle".
const SETTLE: f32 = 5.0;

#[derive(Clone, Copy, Default)]
struct Chan { y: f32, v: f32 }

impl Chan {
    /// Exact critically damped response over `dt` toward a target held constant across the step:
    /// with `d = y - x` and `b = v + w*d`, `y(t) = x + (d + b t) e^(-w t)` and `v(t) = (v - w b t)
    /// e^(-w t)`. Analytic, so it is unconditionally stable at any `w * dt` - unlike the explicit
    /// Euler spring, which rings then diverges once `w * dt` passes ~0.5 (reachable here: a
    /// 120ms setting at 60fps is already `w * dt = 0.69`).
    fn step(&mut self, x: f32, w: f32, dt: f32) -> f32 {
        let d = self.y - x;
        let b = self.v + w * d;
        let e = (-w * dt).exp();
        self.y = x + (d + b * dt) * e;
        self.v = (self.v - w * b * dt) * e;
        self.y
    }
}

/// The filter's state: one channel per component, plus whether it has been primed.
pub(crate) struct Damped2 { s: Chan, x: Chan, y: Chan, on: bool }

impl Damped2 {
    pub(crate) fn new() -> Self {
        Self { s: Chan::default(), x: Chan::default(), y: Chan::default(), on: false }
    }

    /// Filter `cam` and re-clamp the result into the frame. `ms == 0` returns `cam` unchanged (and
    /// re-arms priming, so switching the knob on mid-clip starts from the live pose instead of a
    /// stale one). The first filtered step primes the state to `cam` with zero velocity, so
    /// enabling smoothing never introduces a startup slide from the frame centre.
    /// `dt_ms` is the caller's EXACT step length - never a difference of rounded frame
    /// timestamps, which is the quantization the per-ms rewrite exists to remove.
    pub(crate) fn apply(&mut self, dt_ms: f32, cam: Camera, ms: u32, fw: f32, fh: f32) -> Camera {
        if ms == 0 { self.on = false; return cam; }
        if !self.on {
            self.on = true;
            self.s = Chan { y: cam.scale, v: 0.0 };
            self.x = Chan { y: cam.cx, v: 0.0 };
            self.y = Chan { y: cam.cy, v: 0.0 };
            return cam;
        }
        let dt = dt_ms / 1000.0;
        let w = SETTLE / (ms as f32 / 1000.0);
        let scale = self.s.step(cam.scale, w, dt).max(0.01);
        let (cx, cy) = (self.x.step(cam.cx, w, dt), self.y.step(cam.cy, w, dt));
        let (hw, hh) = (fw / (2.0 * scale), fh / (2.0 * scale));
        Camera { cx: cx.clamp(hw, (fw - hw).max(hw)), cy: cy.clamp(hh, (fh - hh).max(hh)), scale }
    }
}
