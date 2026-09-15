pub const PREFIX: &str = "cubic(";

pub fn parse_cubic(s: &str) -> Option<(f32, f32, f32, f32)> {
    let inner = s.trim().strip_prefix(PREFIX)?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let mut v = [0f32; 4];
    for slot in v.iter_mut() {
        *slot = parts.next()?.trim().parse::<f32>().ok()?;
        if !slot.is_finite() {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Some((v[0].clamp(0.0, 1.0), v[1], v[2].clamp(0.0, 1.0), v[3]))
}

pub fn format_cubic(x1: f32, y1: f32, x2: f32, y2: f32) -> String {
    format!("cubic({x1:.3},{y1:.3},{x2:.3},{y2:.3})")
}

fn bez(a: f32, b: f32, t: f32) -> f32 {
    let (c1, c2, c3) = (3.0 * a, 3.0 * b - 6.0 * a, 3.0 * a - 3.0 * b + 1.0);
    ((c3 * t + c2) * t + c1) * t
}

fn dbez(a: f32, b: f32, t: f32) -> f32 {
    let (c1, c2, c3) = (3.0 * a, 3.0 * b - 6.0 * a, 3.0 * a - 3.0 * b + 1.0);
    (3.0 * c3 * t + 2.0 * c2) * t + c1
}

fn solve_t(x1: f32, x2: f32, x: f32) -> f32 {
    let mut t = x;
    for _ in 0..8 {
        let e = bez(x1, x2, t) - x;
        if e.abs() < 1e-7 {
            return t;
        }
        let d = dbez(x1, x2, t);
        if d.abs() < 1e-6 {
            break;
        }
        t = (t - e / d).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..30 {
        let e = bez(x1, x2, t) - x;
        if e.abs() < 1e-7 {
            return t;
        }
        if e > 0.0 {
            hi = t;
        } else {
            lo = t;
        }
        t = (lo + hi) * 0.5;
    }
    t
}

pub fn eval(x1: f32, y1: f32, x2: f32, y2: f32, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    bez(y1, y2, solve_t(x1, x2, p))
}

#[cfg(test)]
#[path = "cubic_tests.rs"]
mod tests;
