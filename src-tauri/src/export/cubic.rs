/// Wire form of a custom curve: `cubic(x1,y1,x2,y2)`. Kept as a plain string so `Zoom.easing`,
/// `LayoutSeg.easing` and `CameraMove.easing` stay `String` fields - no schema change, and a doc
/// written before custom curves existed parses byte-identically.
pub const PREFIX: &str = "cubic(";

/// Parse `cubic(x1,y1,x2,y2)`. `x1`/`x2` are clamped into `[0, 1]` (CSS's rule - it keeps `x(t)`
/// monotonic, so the solve below always has one answer); `y1`/`y2` stay free so a curve can
/// overshoot. Anything malformed, non-finite, or with the wrong arity is `None`.
pub fn parse_cubic(s: &str) -> Option<(f32, f32, f32, f32)> {
    let inner = s.trim().strip_prefix(PREFIX)?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let mut v = [0f32; 4];
    for slot in v.iter_mut() {
        *slot = parts.next()?.trim().parse::<f32>().ok()?;
        if !slot.is_finite() { return None; }
    }
    if parts.next().is_some() { return None; }
    Some((v[0].clamp(0.0, 1.0), v[1], v[2].clamp(0.0, 1.0), v[3]))
}

/// The canonical wire string for a set of control points - fixed 3-decimal fields so a value that
/// round-trips through `parse_cubic` + this is byte-stable (the frontend writes the same form).
pub fn format_cubic(x1: f32, y1: f32, x2: f32, y2: f32) -> String {
    format!("cubic({x1:.3},{y1:.3},{x2:.3},{y2:.3})")
}

/// One coordinate of the cubic Bezier through P0=(0,0), P1=(a,·), P2=(b,·), P3=(1,1), in Horner
/// form: `3(1-t)^2*t*a + 3(1-t)*t^2*b + t^3`.
fn bez(a: f32, b: f32, t: f32) -> f32 {
    let (c1, c2, c3) = (3.0 * a, 3.0 * b - 6.0 * a, 3.0 * a - 3.0 * b + 1.0);
    ((c3 * t + c2) * t + c1) * t
}

/// `d/dt` of `bez` - Newton's step below needs it.
fn dbez(a: f32, b: f32, t: f32) -> f32 {
    let (c1, c2, c3) = (3.0 * a, 3.0 * b - 6.0 * a, 3.0 * a - 3.0 * b + 1.0);
    (3.0 * c3 * t + 2.0 * c2) * t + c1
}

/// The curve parameter `t` where `x(t) == x`. Newton first (fast, ~3 iterations for a well-formed
/// curve), bisection as the guaranteed fallback - `x(t)` is monotonic for `x1`/`x2` in `[0,1]`, so
/// bisection cannot get stuck even when Newton stalls on a near-zero derivative.
fn solve_t(x1: f32, x2: f32, x: f32) -> f32 {
    let mut t = x;
    for _ in 0..8 {
        let e = bez(x1, x2, t) - x;
        if e.abs() < 1e-7 { return t; }
        let d = dbez(x1, x2, t);
        if d.abs() < 1e-6 { break; }
        t = (t - e / d).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..30 {
        let e = bez(x1, x2, t) - x;
        if e.abs() < 1e-7 { return t; }
        if e > 0.0 { hi = t; } else { lo = t; }
        t = (lo + hi) * 0.5;
    }
    t
}

/// Evaluate a CSS-semantics cubic-bezier easing at progress `p` (input clamped to `[0,1]`).
/// Endpoints are exact by construction: `p == 0` and `p == 1` short-circuit.
pub fn eval(x1: f32, y1: f32, x2: f32, y2: f32, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    if p <= 0.0 { return 0.0; }
    if p >= 1.0 { return 1.0; }
    bez(y1, y2, solve_t(x1, x2, p))
}

#[cfg(test)]
#[path = "cubic_tests.rs"]
mod tests;
