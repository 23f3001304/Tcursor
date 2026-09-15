pub const REF_W: f64 = 1920.0;

const SUB_MS: f64 = 1.0;

const REF_STEP_MS: f64 = 1000.0 / 60.0;

const VEL_A: f64 = 0.35;

const K: f64 = 2.0;

const DEAD: f64 = 0.4;

pub const MAX_DEG: f32 = 6.0;

const W: f64 = 18.0;
const Z: f64 = 0.5;

const MAX_CATCHUP_MS: f64 = 100.0;

#[derive(Default)]
pub struct Tilt {
    vx: f64,
    vy: f64,
    angle: f64,
    avel: f64,
    px: f64,
    py: f64,
    acc: f64,
    primed: bool,
}

impl Tilt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn angle_deg(&self) -> f32 {
        self.angle as f32
    }

    pub fn step(&mut self, x: f32, y: f32, dt_ms: f32, max_deg: f32) -> f32 {
        let (x, y, dt) = (x as f64, y as f64, dt_ms as f64);
        if !self.primed || !(dt > 0.0) {
            self.px = x;
            self.py = y;
            self.primed = true;
            return self.angle_deg();
        }
        let (ivx, ivy) = ((x - self.px) / dt, (y - self.py) / dt);
        self.px = x;
        self.py = y;
        self.acc = (self.acc + dt).min(MAX_CATCHUP_MS);
        let a = 1.0 - (1.0 - VEL_A).powf(SUB_MS / REF_STEP_MS);
        let h = SUB_MS / 1000.0;
        let max = max_deg.max(0.0) as f64;
        while self.acc >= SUB_MS {
            self.acc -= SUB_MS;
            self.vx += (ivx - self.vx) * a;
            self.vy += (ivy - self.vy) * a;
            let target = target_deg(self.vx, self.vy, max);
            self.avel += (-W * W * (self.angle - target) - 2.0 * Z * W * self.avel) * h;
            self.angle += self.avel * h;
        }
        self.angle_deg()
    }
}

fn target_deg(vx: f64, vy: f64, max: f64) -> f64 {
    let speed = vx.hypot(vy);
    if speed <= DEAD {
        return 0.0;
    }
    let gate = (speed - DEAD) / speed;
    (K * (vx + 0.5 * vy) * gate).clamp(-max, max)
}

pub fn max_deg(tilt: f32) -> f32 {
    MAX_DEG * tilt.clamp(0.0, 1.0)
}

pub fn ref_scale(screen_w: u32) -> f32 {
    REF_W as f32 / screen_w.max(1) as f32
}

#[cfg(test)]
#[path = "tilt_tests.rs"]
mod tests;
