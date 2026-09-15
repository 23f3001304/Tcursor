use crate::edit::model::CameraMove;
use crate::export::camera::ease;
use crate::export::render::fromedit::easing_from;
use crate::export::types::Easing;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub round: Option<f32>,
}

pub const KF_BLEND_MS: u32 = 350;

struct Kf {
    t_ms: u32,
    x: f32,
    y: f32,
    size: f32,
    round: Option<f32>,
    easing: Easing,
}

pub fn shape_round(shape: &str, roundness: f32) -> Option<f32> {
    match shape {
        "circle" => Some(0.5),
        "rect" => Some(0.0),
        "rounded" => Some(roundness.clamp(0.0, 0.5)),
        _ => None,
    }
}

pub struct CameraMoveTrack {
    kfs: Vec<Kf>,
}

impl CameraMoveTrack {
    pub fn from_doc(moves: &[CameraMove]) -> Self {
        let mut kfs: Vec<Kf> = moves
            .iter()
            .map(|m| Kf {
                t_ms: m.t_ms,
                x: m.x,
                y: m.y,
                size: m.size,
                round: shape_round(&m.shape, m.roundness),
                easing: easing_from(&m.easing, Easing::Smooth),
            })
            .collect();
        kfs.sort_by_key(|k| k.t_ms);
        Self { kfs }
    }

    pub fn span(&self) -> Option<(u32, u32)> {
        Some((
            self.kfs.first()?.t_ms.saturating_sub(KF_BLEND_MS),
            self.kfs.last()?.t_ms.saturating_add(KF_BLEND_MS),
        ))
    }

    pub fn sample(&self, t_ms: u32, live: Option<CamPose>) -> Option<CamPose> {
        let (entry, exit) = self.span()?;
        let (first, last) = (self.kfs.first()?, self.kfs.last()?);
        if t_ms < entry || t_ms > exit {
            return None;
        }
        let pose = |k: &Kf| CamPose {
            x: k.x,
            y: k.y,
            size: k.size,
            round: k.round.or(live.and_then(|l| l.round)),
        };
        if t_ms < first.t_ms {
            let win = first.t_ms - entry;
            return Some(match live {
                Some(l) if win > 0 => mix(
                    l,
                    pose(first),
                    ease(first.easing, (t_ms - entry) as f32 / win as f32),
                ),
                _ => pose(first),
            });
        }
        if t_ms > last.t_ms {
            return Some(match live {
                Some(l) => mix(
                    pose(last),
                    l,
                    ease(last.easing, (t_ms - last.t_ms) as f32 / KF_BLEND_MS as f32),
                ),
                None => pose(last),
            });
        }
        if t_ms == last.t_ms {
            return Some(pose(last));
        }

        let bi = self.kfs.iter().position(|k| k.t_ms > t_ms).unwrap();
        let (a, b) = (&self.kfs[bi - 1], &self.kfs[bi]);
        if b.t_ms == a.t_ms {
            return Some(pose(b));
        }
        let f = ease(b.easing, (t_ms - a.t_ms) as f32 / (b.t_ms - a.t_ms) as f32);
        Some(mix(pose(a), pose(b), f))
    }
}

fn mix(a: CamPose, b: CamPose, f: f32) -> CamPose {
    let round = match (a.round, b.round) {
        (Some(ra), Some(rb)) => Some(ra + (rb - ra) * f),
        (ra, rb) => ra.or(rb),
    };
    CamPose {
        x: a.x + (b.x - a.x) * f,
        y: a.y + (b.y - a.y) * f,
        size: a.size + (b.size - a.size) * f,
        round,
    }
}

#[cfg(test)]
#[path = "moves_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "moves_span_tests.rs"]
mod span_tests;
