use crate::export::types::Easing;

pub const PREFIX: &str = "spring(";

pub const STIFFNESS: (f32, f32) = (1.0, 2000.0);
pub const DAMPING: (f32, f32) = (0.0, 200.0);
pub const MASS: (f32, f32) = (0.1, 10.0);

const ZETA_MIN: f32 = 0.05;

const LN_EPS: f32 = 6.907_755;

const CRIT: f32 = 1e-3;

fn zeta(stiffness: f32, damping: f32, mass: f32) -> f32 {
    let k = stiffness.clamp(STIFFNESS.0, STIFFNESS.1);
    let (c, m) = (
        damping.clamp(DAMPING.0, DAMPING.1),
        mass.clamp(MASS.0, MASS.1),
    );
    (c / (2.0 * (k * m).sqrt())).max(ZETA_MIN)
}

fn settle_u(z: f32) -> f32 {
    LN_EPS
        / if z <= 1.0 {
            z
        } else {
            z - (z * z - 1.0).max(0.0).sqrt()
        }
}

fn resp(z: f32, u: f32) -> f32 {
    if (z - 1.0).abs() <= CRIT {
        return 1.0 - (1.0 + u) * (-u).exp();
    }
    if z < 1.0 {
        let d = (1.0 - z * z).sqrt();
        return 1.0 - (-z * u).exp() * ((d * u).cos() + z / d * (d * u).sin());
    }
    let s = (z * z - 1.0).max(0.0).sqrt();
    let (r1, r2) = (-(z - s), -(z + s));
    1.0 - (r2 * (r1 * u).exp() - r1 * (r2 * u).exp()) / (r2 - r1)
}

pub fn spring(stiffness: f32, damping: f32, mass: f32, p: f32) -> f32 {
    if !p.is_finite() || p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let (z, us) = {
        let z = zeta(stiffness, damping, mass);
        (z, settle_u(z))
    };
    resp(z, p * us) + p * (1.0 - resp(z, us))
}

pub fn eval(e: Easing, p: f32) -> f32 {
    match e {
        Easing::Spring {
            stiffness,
            damping,
            mass,
        } => spring(stiffness, damping, mass, p),
        _ => p,
    }
}

pub fn parse_spring(s: &str) -> Option<(f32, f32, f32)> {
    let inner = s.trim().strip_prefix(PREFIX)?.strip_suffix(')')?;
    let mut v = [0f32; 3];
    let mut n = 0;
    for part in inner.split(',') {
        if n == 3 {
            return None;
        }
        v[n] = part.trim().parse::<f32>().ok()?;
        if !v[n].is_finite() {
            return None;
        }
        n += 1;
    }
    if n < 2 {
        return None;
    }
    if n == 2 {
        v[2] = 1.0;
    }
    Some((
        v[0].clamp(STIFFNESS.0, STIFFNESS.1),
        v[1].clamp(DAMPING.0, DAMPING.1),
        v[2].clamp(MASS.0, MASS.1),
    ))
}

pub fn format_spring(stiffness: f32, damping: f32, mass: f32) -> String {
    format!("spring({stiffness:.3},{damping:.3},{mass:.3})")
}

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;
