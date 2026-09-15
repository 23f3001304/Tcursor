use crate::export::camera::moves::CamPose;
use crate::export::scene::Panel;
use crate::export::types::{Camera, Easing, FramePoint, ZoomConfig, ZoomRegion};

pub(crate) fn ease(e: Easing, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    match e {
        Easing::Linear => p,
        Easing::Smooth => p * p * (3.0 - 2.0 * p),
        Easing::Spring { .. } => crate::export::spring::eval(e, p),
        Easing::EaseIn => p * p,
        Easing::EaseOut => p * (2.0 - p),
        Easing::EaseInOut => {
            if p < 0.5 {
                2.0 * p * p
            } else {
                1.0 - 2.0 * (1.0 - p) * (1.0 - p)
            }
        }
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::eval(x1, y1, x2, y2, p),
        Easing::Keys(ref k) => crate::export::keys::eval(k, p),
    }
}

pub fn static_cam_pose(panel: &Panel, ow: f32, oh: f32) -> CamPose {
    let r = panel.rect;
    CamPose {
        x: (r.x + r.w * 0.5) / ow,
        y: (r.y + r.h * 0.5) / oh,
        size: r.h / oh,
        round: Some(panel.radius / r.w.min(r.h).max(0.001)),
    }
}

pub(crate) fn fit_durations(zi: u32, zo: u32, span: u32) -> (u32, u32) {
    let total = zi + zo;
    if total <= span || total == 0 {
        return (zi, zo);
    }
    let f = span as f32 / total as f32;
    ((zi as f32 * f) as u32, (zo as f32 * f) as u32)
}

pub struct CameraSim {
    frame_w: u32,
    frame_h: u32,
    cx: f32,
    cy: f32,
    scale: f32,
    driver: Option<usize>,
    driver_zoom_out_ms: u32,
    driver_easing: Easing,
    transition: Option<handoff::Transition>,
    vel: (f32, f32, f32),
    stepped: bool,
    smooth: smoothing::Damped2,
}

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self {
            frame_w,
            frame_h,
            cx: frame_w as f32 / 2.0,
            cy: frame_h as f32 / 2.0,
            scale: 1.0,
            driver: None,
            driver_zoom_out_ms: 450,
            driver_easing: Easing::Smooth,
            transition: None,
            vel: (0.0, 0.0, 0.0),
            stepped: false,
            smooth: smoothing::Damped2::new(),
        }
    }

    fn winner(regions: &[ZoomRegion], t_ms: u32) -> Option<usize> {
        regions
            .iter()
            .enumerate()
            .filter(|(_, r)| t_ms >= r.start_ms && t_ms <= r.end_ms)
            .max_by_key(|(i, r)| (r.layer, *i))
            .map(|(i, _)| i)
    }

    fn remaining_zoom_in(r: &ZoomRegion, t_ms: u32) -> u32 {
        let span = r.end_ms.saturating_sub(r.start_ms).max(1);
        let (zi, _) = fit_durations(r.zoom_in_ms, r.zoom_out_ms, span);
        let zin_end = r.start_ms + zi;
        if t_ms < zin_end {
            zin_end - t_ms
        } else {
            r.zoom_in_ms
        }
    }

    pub fn step(
        &mut self,
        t_ms: u32,
        dt_ms: f32,
        cursor: FramePoint,
        regions: &[ZoomRegion],
        cfg: &ZoomConfig,
    ) -> Camera {
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        let prev = Camera {
            cx: self.cx,
            cy: self.cy,
            scale: self.scale,
        };
        let dt = if dt_ms.is_finite() {
            dt_ms.clamp(0.1, 1000.0)
        } else {
            follow::REF_STEP_MS
        };
        let winner_idx = Self::winner(regions, t_ms);

        if winner_idx != self.driver {
            if self.driver.is_some() {
                let (dur_ms, easing) = match winner_idx {
                    Some(i) => (
                        Self::remaining_zoom_in(&regions[i], t_ms),
                        regions[i].easing,
                    ),
                    None => (self.driver_zoom_out_ms, self.driver_easing),
                };
                self.transition = Some(handoff::Transition::open(
                    prev,
                    self.vel,
                    dt,
                    dur_ms as f32,
                    easing,
                ));
            } else {
                self.transition = None;
            }
            self.driver = winner_idx;
        }
        if let Some(i) = winner_idx {
            self.driver_zoom_out_ms = regions[i].zoom_out_ms;
            self.driver_easing = regions[i].easing;
        }

        let (natural_scale, natural_cx, natural_cy) = match winner_idx {
            None => (1.0, fw / 2.0, fh / 2.0),
            Some(i) => {
                let r = &regions[i];
                let span = r.end_ms.saturating_sub(r.start_ms).max(1);
                let (zi, zo) = fit_durations(r.zoom_in_ms, r.zoom_out_ms, span);
                let (zin_end, zout_start) = (r.start_ms + zi, r.end_ms.saturating_sub(zo));
                let s = r.target_scale;
                let (tx, ty) = follow::aim(r, cursor);
                if t_ms < zin_end {
                    let e = if self.transition.is_some() {
                        1.0
                    } else {
                        ease(r.easing, (t_ms - r.start_ms) as f32 / zi.max(1) as f32)
                    };
                    (
                        1.0 + (s - 1.0) * e,
                        fw / 2.0 + (tx - fw / 2.0) * e,
                        fh / 2.0 + (ty - fh / 2.0) * e,
                    )
                } else {
                    let scale = if t_ms >= zout_start {
                        1.0 + (s - 1.0)
                            * ease(r.easing_out, (r.end_ms - t_ms) as f32 / zo.max(1) as f32)
                    } else {
                        s
                    };
                    let k = follow::damping(cfg.follow_damping, dt);
                    (
                        scale,
                        self.cx + (tx - self.cx) * k,
                        self.cy + (ty - self.cy) * k,
                    )
                }
            }
        };

        let natural = Camera {
            cx: natural_cx,
            cy: natural_cy,
            scale: natural_scale,
        };
        let out = match self
            .transition
            .as_mut()
            .and_then(|tr| tr.blend(dt, natural))
        {
            Some(blended) => blended,
            None => {
                self.transition = None;
                natural
            }
        };
        (self.scale, self.cx, self.cy) = (out.scale.max(1.0), out.cx, out.cy);

        let (half_w, half_h) = (
            fw / (2.0 * self.scale.max(0.01)),
            fh / (2.0 * self.scale.max(0.01)),
        );
        self.cx = self.cx.clamp(half_w, (fw - half_w).max(half_w));
        self.cy = self.cy.clamp(half_h, (fh - half_h).max(half_h));
        self.vel = if self.stepped {
            (
                (self.cx - prev.cx) / dt,
                (self.cy - prev.cy) / dt,
                (self.scale - prev.scale) / dt,
            )
        } else {
            (0.0, 0.0, 0.0)
        };
        self.stepped = true;
        let raw = Camera {
            cx: self.cx,
            cy: self.cy,
            scale: self.scale,
        };
        self.smooth.apply(dt, raw, cfg.smoothing_ms, fw, fh)
    }
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod cursor_tests;
#[cfg(test)]
#[path = "jank_probe_tests.rs"]
mod jank_probe_tests;
#[cfg(test)]
#[path = "motion_tests.rs"]
mod motion_tests;
#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;

pub mod autozoom;
pub(crate) mod follow;
mod handoff;
pub mod manual;
pub mod moves;
mod smoothing;
