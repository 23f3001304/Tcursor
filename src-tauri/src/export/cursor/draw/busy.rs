use serde::{Deserialize, Serialize};

const CYCLE_MS: f64 = 1000.0;

const FLIP_HOLD: f64 = 0.7;

const PULSE_PEAK: f64 = 1.06;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BusyAnim {
    Spin,
    Flip,
    Pulse,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BusySpec {
    pub anim: BusyAnim,
    pub fps: f32,
    #[serde(default)]
    pub frames: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BusyPose {
    pub frame: u32,
    pub angle_deg: f32,
    pub scale: f32,
}

impl BusyPose {
    pub fn still() -> Self {
        Self {
            frame: 0,
            angle_deg: 0.0,
            scale: 1.0,
        }
    }
    pub fn is_identity(&self) -> bool {
        self.angle_deg == 0.0 && self.scale == 1.0
    }
}

pub fn busy_pose(spec: &BusySpec, t_ms: u32) -> BusyPose {
    let t = t_ms as f64;
    if spec.frames > 0 {
        let fps = (spec.fps as f64).max(0.001);
        let i = (t / 1000.0 * fps).floor().max(0.0) as u32 % spec.frames;
        return BusyPose {
            frame: i,
            ..BusyPose::still()
        };
    }
    match spec.anim {
        BusyAnim::Spin => BusyPose {
            angle_deg: (360.0 * (t / 1000.0 * spec.fps as f64 / 24.0)).rem_euclid(360.0) as f32,
            ..BusyPose::still()
        },
        BusyAnim::Flip => BusyPose {
            angle_deg: flip_angle(t) as f32,
            ..BusyPose::still()
        },
        BusyAnim::Pulse => BusyPose {
            scale: pulse_scale(t) as f32,
            ..BusyPose::still()
        },
    }
}

fn flip_angle(t_ms: f64) -> f64 {
    let cycles = (t_ms / CYCLE_MS).floor();
    let u = t_ms / CYCLE_MS - cycles;
    let turned = if u < FLIP_HOLD {
        0.0
    } else {
        180.0 * ease_in_out((u - FLIP_HOLD) / (1.0 - FLIP_HOLD))
    };
    (cycles * 180.0 + turned).rem_euclid(360.0)
}

fn pulse_scale(t_ms: f64) -> f64 {
    let u = t_ms / CYCLE_MS;
    1.0 + (PULSE_PEAK - 1.0) * bump(u - u.floor())
}

fn ease_in_out(u: f64) -> f64 {
    (1.0 - (std::f64::consts::PI * u.clamp(0.0, 1.0)).cos()) / 2.0
}

fn bump(u: f64) -> f64 {
    (1.0 - (std::f64::consts::TAU * u.clamp(0.0, 1.0)).cos()) / 2.0
}

#[cfg(test)]
#[path = "busy_tests.rs"]
mod tests;
