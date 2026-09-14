// The very slight lean a cursor takes on when it is thrown across the screen, and the single
// overshoot it corrects through when it comes to rest. Other editors do this; a recorded pointer
// never does it by itself, because a mouse cursor has no orientation to lag behind its own motion.
//
// Two filters in series. First a low-pass on the SMOOTHED cursor's own velocity, so the lean
// follows the gesture rather than one stray sample. Then a lightly under-damped spring chasing the
// angle that velocity asks for - the spring is what puts the overshoot in: cut the target to zero
// and it swings past once, by about a degree, roughly 200 ms after the stop, then settles.
//
// Both run on a FIXED 1 ms substep grid with the leftover carried, which is what makes a 30 fps
// export match a 60 fps one at the same instant. A first-order low-pass can be re-based to any
// step exactly (`follow::damping` does it in closed form); a second-order spring chasing a moving
// target cannot, so the step is made small and constant instead.
//
// f64 throughout, for the same reason `busy.rs` is: `src/editor/stage/cursorTilt.ts` is the live
// preview's mirror of this file and has no f32, and the two must agree to under a thousandth of a
// degree or pausing (which swaps in the backend's own frame) would visibly twitch the cursor.

/// The screen width every tilt speed is measured against. Speeds go through this before they reach
/// the filter, so one gesture leans the same whether it was recorded at 1080p or at 4K - a raw
/// px/ms threshold would mean "twice as fast" on the bigger capture for exactly the same hand
/// movement. It is also the unit `src/editor/stage/cursorTilt.ts` works in, which is what lets the
/// preview mirror the filter without knowing the recording's pixel size.
pub const REF_W: f64 = 1920.0;

/// The filter's fixed integration step, in ms.
const SUB_MS: f64 = 1.0;
/// One 60 fps frame in ms - the basis `VEL_A` is quoted in, exactly as `follow::REF_STEP_MS`.
const REF_STEP_MS: f64 = 1000.0 / 60.0;
/// Fraction of the velocity error closed in one 60 fps frame (`follow::damping`'s convention,
/// re-derived here in f64 so the TS mirror matches bit for bit rather than to f32).
const VEL_A: f64 = 0.35;
/// Degrees of lean per reference px/ms of travel past the dead zone.
const K: f64 = 2.0;
/// Nothing leans below this speed (reference px per ms, i.e. ~400 px/s at 1080p), so ordinary
/// pointing - reaching for a button, nudging a slider - stays perfectly upright.
const DEAD: f64 = 0.4;
/// The lean a full-speed sweep SETTLES at when `tilt = 1`, in degrees; `tilt` scales it. The cap
/// is on the target, so the spring still swings ~16% past it on the way in - that swing is the
/// effect, not a leak, and it is the only thing that ever exceeds this number.
pub const MAX_DEG: f32 = 6.0;
/// Spring natural frequency (rad/s) and damping ratio. `Z = 0.5` overshoots by ~16% of the lean it
/// is returning from (about a degree from a full one) and `W = 18` puts that peak at
/// `pi / (W * sqrt(1 - Z^2))` = 201 ms; the next swing back is ~0.16 deg, which is under a tenth of
/// a pixel at the far corner of a rendered cursor - one visible overshoot, then settled.
const W: f64 = 18.0;
const Z: f64 = 0.5;
/// The most elapsed time one call may integrate. Only the live preview can hand this a big `dt`
/// (a stalled tab, a scrub); the export always steps `OUT_STEP_MS`. Capping it bounds the substep
/// loop instead of letting a 10-second stall run 10,000 iterations.
const MAX_CATCHUP_MS: f64 = 100.0;

/// The lean filter's state: the smoothed velocity it reads, the spring it drives, and the previous
/// position it differences. One per `Cursor`; `reset` returns it to upright and unprimed.
#[derive(Default)]
pub struct Tilt {
    vx: f64, vy: f64,      // smoothed velocity, reference px per ms
    angle: f64, avel: f64, // spring state: degrees, and degrees per second
    px: f64, py: f64,      // the previous position, reference px
    acc: f64,              // elapsed ms not yet integrated (always < SUB_MS after a step)
    primed: bool,          // has a previous position to difference against
}

impl Tilt {
    pub fn new() -> Self { Self::default() }

    /// Back to upright and unprimed - what `Cursor::reset` calls at a cut, so the cursor does not
    /// arrive at the far side of a splice still leaning from the gesture before it.
    pub fn reset(&mut self) { *self = Self::default(); }

    /// The current lean in degrees, clockwise-positive (the sense `blit_transformed` rotates in).
    pub fn angle_deg(&self) -> f32 { self.angle as f32 }

    /// Advance by one frame of `dt_ms` with the cursor at `(x, y)` in REFERENCE px (frame px times
    /// `ref_scale`), and return the new lean. `max_deg` is the cap this frame's `tilt` setting
    /// allows (`max_deg()` below); passing 0 pins the target at 0, so the spring unwinds rather
    /// than freezing mid-lean. The first call (and any non-positive `dt_ms`) only records the
    /// position: there is no velocity to read yet.
    pub fn step(&mut self, x: f32, y: f32, dt_ms: f32, max_deg: f32) -> f32 {
        let (x, y, dt) = (x as f64, y as f64, dt_ms as f64);
        if !self.primed || !(dt > 0.0) {
            self.px = x; self.py = y; self.primed = true;
            return self.angle_deg();
        }
        let (ivx, ivy) = ((x - self.px) / dt, (y - self.py) / dt);
        self.px = x; self.py = y;
        self.acc = (self.acc + dt).min(MAX_CATCHUP_MS);
        let a = 1.0 - (1.0 - VEL_A).powf(SUB_MS / REF_STEP_MS);
        let h = SUB_MS / 1000.0;
        let max = max_deg.max(0.0) as f64;
        while self.acc >= SUB_MS {
            self.acc -= SUB_MS;
            self.vx += (ivx - self.vx) * a;
            self.vy += (ivy - self.vy) * a;
            let target = target_deg(self.vx, self.vy, max);
            // Semi-implicit Euler: the velocity is updated first and the angle integrates the NEW
            // velocity, which is what keeps a lightly-damped spring from gaining energy step over
            // step the way the explicit form does.
            self.avel += (-W * W * (self.angle - target) - 2.0 * Z * W * self.avel) * h;
            self.angle += self.avel * h;
        }
        self.angle_deg()
    }
}

/// The angle a velocity of `(vx, vy)` reference px/ms asks for, capped at `max`.
///
/// The sign follows the HORIZONTAL direction of travel - moving right leans the sprite clockwise,
/// left anticlockwise - because that is the lean a held object takes on when it is swung sideways.
/// A purely vertical throw has no such direction, so it leans by its vertical component at half
/// weight rather than not at all. `gate` fades the lean in from the dead-zone edge instead of
/// switching it on: a cursor hovering right at the threshold must not flicker between upright and
/// leaning.
fn target_deg(vx: f64, vy: f64, max: f64) -> f64 {
    let speed = vx.hypot(vy);
    if speed <= DEAD { return 0.0; }
    let gate = (speed - DEAD) / speed;
    (K * (vx + 0.5 * vy) * gate).clamp(-max, max)
}

/// The cap the `tilt` setting (0..1) allows. 0 is off outright - the caller skips the filter
/// entirely, so the angle stays 0 and nothing is integrated.
pub fn max_deg(tilt: f32) -> f32 { MAX_DEG * tilt.clamp(0.0, 1.0) }

/// Frame px -> reference px for a capture `screen_w` wide (see `REF_W`).
pub fn ref_scale(screen_w: u32) -> f32 { REF_W as f32 / screen_w.max(1) as f32 }

#[cfg(test)]
#[path = "tilt_tests.rs"]
mod tests;
