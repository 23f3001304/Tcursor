// A real damped harmonic oscillator behind `Easing::Spring`, plus its wire form
// `spring(stiffness,damping[,mass])`. Mirrored in TS by `src/lib/spring.ts`; the export is the
// source of truth. Full rationale (in particular the settle-time remap and what it costs) lives
// in docs/api/src-tauri/src/export/spring.md - read that before changing a constant here.
use crate::export::types::Easing;

pub const PREFIX: &str = "spring(";
/// Ranges the wire and the UI agree on. Stiffness/mass are positive by definition; damping may be
/// 0 (a spring that never stops ringing), which `ZETA_MIN` then makes finite.
pub const STIFFNESS: (f32, f32) = (1.0, 2000.0);
pub const DAMPING: (f32, f32) = (0.0, 200.0);
pub const MASS: (f32, f32) = (0.1, 10.0);
/// Floor on the damping ratio. At zeta 0 the oscillator never settles, so there is no window to
/// normalise onto; 0.05 is ~15 visible oscillations, which is as "boing" as this can express.
const ZETA_MIN: f32 = 0.05;
/// `ln(1 / 1e-3)` - the settle band is "the slowest decaying mode has fallen to 0.1%".
const LN_EPS: f32 = 6.907_755;
/// Half-width of the band around zeta = 1 evaluated with the critically damped form, where the
/// under/overdamped ones both divide by ~0.
const CRIT: f32 = 1e-3;

/// The damping ratio the curve's shape is entirely a function of (see `spring`).
fn zeta(stiffness: f32, damping: f32, mass: f32) -> f32 {
    let k = stiffness.clamp(STIFFNESS.0, STIFFNESS.1);
    let (c, m) = (damping.clamp(DAMPING.0, DAMPING.1), mass.clamp(MASS.0, MASS.1));
    (c / (2.0 * (k * m).sqrt())).max(ZETA_MIN)
}

/// `w0 * t` at which the slowest mode has decayed to `1e-3` - the instant `p = 1` maps to.
/// Continuous at zeta = 1 (both branches give `w0`), and bounded because zeta is floored.
fn settle_u(z: f32) -> f32 {
    LN_EPS / if z <= 1.0 { z } else { z - (z * z - 1.0).max(0.0).sqrt() }
}

/// Unit-step response of the oscillator at `u = w0 * t`, from rest at 0 toward 1.
fn resp(z: f32, u: f32) -> f32 {
    if (z - 1.0).abs() <= CRIT { return 1.0 - (1.0 + u) * (-u).exp(); }
    if z < 1.0 {
        let d = (1.0 - z * z).sqrt();
        return 1.0 - (-z * u).exp() * ((d * u).cos() + z / d * (d * u).sin());
    }
    let s = (z * z - 1.0).max(0.0).sqrt();
    let (r1, r2) = (-(z - s), -(z + s));
    1.0 - (r2 * (r1 * u).exp() - r1 * (r2 * u).exp()) / (r2 - r1)
}

/// The spring easing at progress `p`: the oscillator's step response, its own settle time remapped
/// onto `[0, 1]`, with the sub-0.1% residue at the far end taken out linearly so `p = 1` is exactly
/// 1.0. `p = 0` is exactly 0. Underdamped params overshoot; critical and overdamped are monotone.
pub fn spring(stiffness: f32, damping: f32, mass: f32, p: f32) -> f32 {
    if !p.is_finite() || p <= 0.0 { return 0.0; }
    if p >= 1.0 { return 1.0; }
    let (z, us) = { let z = zeta(stiffness, damping, mass); (z, settle_u(z)) };
    resp(z, p * us) + p * (1.0 - resp(z, us))
}

/// `Easing::Spring`'s evaluator - the arm `camera::ease` delegates to.
pub fn eval(e: Easing, p: f32) -> f32 {
    match e {
        Easing::Spring { stiffness, damping, mass } => spring(stiffness, damping, mass, p),
        _ => p,
    }
}

/// Parse `spring(stiffness,damping)` or `spring(stiffness,damping,mass)`; mass defaults to 1.
/// Values are clamped into the published ranges. Malformed or wrong-arity input is `None`, so the
/// caller falls through to its own default exactly as it does for a bad `cubic(...)`.
pub fn parse_spring(s: &str) -> Option<(f32, f32, f32)> {
    let inner = s.trim().strip_prefix(PREFIX)?.strip_suffix(')')?;
    let mut v = [0f32; 3];
    let mut n = 0;
    for part in inner.split(',') {
        if n == 3 { return None; }
        v[n] = part.trim().parse::<f32>().ok()?;
        if !v[n].is_finite() { return None; }
        n += 1;
    }
    if n < 2 { return None; }
    if n == 2 { v[2] = 1.0; }
    Some((v[0].clamp(STIFFNESS.0, STIFFNESS.1), v[1].clamp(DAMPING.0, DAMPING.1),
        v[2].clamp(MASS.0, MASS.1)))
}

/// The canonical wire string - fixed 3-decimal fields and always all three, so a value that
/// round-trips through `parse_spring` + this is byte-stable (the frontend writes the same form).
pub fn format_spring(stiffness: f32, damping: f32, mass: f32) -> String {
    format!("spring({stiffness:.3},{damping:.3},{mass:.3})")
}

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;
