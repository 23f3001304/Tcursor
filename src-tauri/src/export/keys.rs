use std::cmp::Ordering;

pub const PREFIX: &str = "keys(";

pub const MAX: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    Bezier,
    Hold,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub t: f32,
    pub v: f32,
    pub in_: [f32; 2],
    pub out: [f32; 2],
    pub mode: KeyMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keys {
    pub n: u8,
    pub keys: [Key; MAX],
}

const ZERO: Key = Key {
    t: 0.0,
    v: 0.0,
    in_: [0.0; 2],
    out: [0.0; 2],
    mode: KeyMode::Bezier,
};

impl Keys {
    pub fn slice(&self) -> &[Key] {
        &self.keys[..(self.n as usize).min(MAX)]
    }
}

fn mode_of(s: &str) -> Option<KeyMode> {
    match s {
        "b" => Some(KeyMode::Bezier),
        "h" => Some(KeyMode::Hold),
        "l" => Some(KeyMode::Linear),
        _ => None,
    }
}

fn mode_ch(m: KeyMode) -> char {
    match m {
        KeyMode::Bezier => 'b',
        KeyMode::Hold => 'h',
        KeyMode::Linear => 'l',
    }
}

fn r3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

pub fn parse_keys(s: &str) -> Option<Keys> {
    let inner = s.trim().strip_prefix(PREFIX)?.strip_suffix(')')?;
    let mut keys = [ZERO; MAX];
    let mut n = 0usize;
    for part in inner.split(',') {
        if n == MAX {
            return None;
        }
        let mut f = part.split_whitespace();
        let mut num = [0f32; 6];
        for slot in num.iter_mut() {
            *slot = f.next()?.parse::<f32>().ok()?;
            if !slot.is_finite() {
                return None;
            }
        }
        let mode = mode_of(f.next()?)?;
        if f.next().is_some() {
            return None;
        }
        keys[n] = Key {
            t: num[0],
            v: num[1],
            in_: [num[2], num[3]],
            out: [num[4], num[5]],
            mode,
        };
        n += 1;
    }
    if n < 2 {
        return None;
    }
    keys[..n].sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(Ordering::Equal));
    if r3(keys[0].t) != 0.0 || r3(keys[n - 1].t) != 1.0 {
        return None;
    }
    if !keys[..n].windows(2).all(|w| r3(w[1].t) > r3(w[0].t)) {
        return None;
    }
    for i in 0..n {
        let back = if i == 0 {
            0.0
        } else {
            keys[i].t - keys[i - 1].t
        };
        let fwd = if i + 1 == n {
            0.0
        } else {
            keys[i + 1].t - keys[i].t
        };
        keys[i].in_[0] = keys[i].in_[0].clamp(-back, 0.0);
        keys[i].out[0] = keys[i].out[0].clamp(0.0, fwd);
        if i == 0 {
            keys[i].in_ = [0.0, 0.0];
        }
        if i + 1 == n {
            keys[i].out = [0.0, 0.0];
        }
    }
    Some(Keys { n: n as u8, keys })
}

fn f3(x: f32) -> String {
    let s = format!("{x:.3}");
    if s == "-0.000" {
        "0.000".to_string()
    } else {
        s
    }
}

pub fn format_keys(k: &Keys) -> String {
    let mut s = String::from(PREFIX);
    for (i, key) in k.slice().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{} {} {} {} {} {} {}",
            f3(key.t),
            f3(key.v),
            f3(key.in_[0]),
            f3(key.in_[1]),
            f3(key.out[0]),
            f3(key.out[1]),
            mode_ch(key.mode)
        ));
    }
    s.push(')');
    s
}

fn cubic_at(p0: f32, p1: f32, p2: f32, p3: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    u * u * u * p0 + 3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s * p3
}

fn bez_seg(a: &Key, b: &Key, p: f32) -> f32 {
    let (x1, y1) = (a.t + a.out[0], a.v + a.out[1]);
    let (x2, y2) = (b.t + b.in_[0], b.v + b.in_[1]);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    let mut s = 0.5f32;
    for _ in 0..20 {
        s = (lo + hi) * 0.5;
        if cubic_at(a.t, x1, x2, b.t, s) < p {
            lo = s;
        } else {
            hi = s;
        }
    }
    cubic_at(a.v, y1, y2, b.v, s)
}

pub fn eval(k: &Keys, p: f32) -> f32 {
    let ks = k.slice();
    if ks.len() < 2 {
        return ks.first().map_or(p, |a| a.v);
    }
    let p = p.clamp(0.0, 1.0);
    let mut i = 0usize;
    while i + 2 < ks.len() && p >= ks[i + 1].t {
        i += 1;
    }
    let (a, b) = (ks[i], ks[i + 1]);
    let span = b.t - a.t;
    if span <= 0.0 {
        return b.v;
    }
    match a.mode {
        KeyMode::Hold => a.v,
        KeyMode::Linear => a.v + (b.v - a.v) * ((p - a.t) / span),
        KeyMode::Bezier => bez_seg(&a, &b, p),
    }
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
