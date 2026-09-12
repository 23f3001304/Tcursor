use crate::export::camera::moves::CamPose;
use crate::export::types::{Camera, Easing, FramePoint, RectF, ZoomConfig, ZoomRegion};

/// Normalized easing curve `[0,1] -> [0,1]`. `Smooth` = smoothstep; `Linear` = identity;
/// `Spring` = ease-out-back (a small overshoot past 1 near the end, then settle);
/// `EaseIn`/`EaseOut`/`EaseInOut` = quadratic accelerate / decelerate / both. `pub(crate)`
/// so `fx_state`'s `SpotlightSim` can reuse the same curves for its own handoff transitions.
pub(crate) fn ease(e: Easing, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    match e {
        Easing::Linear => p,
        Easing::Smooth => p * p * (3.0 - 2.0 * p),
        Easing::Spring { .. } => { let c = 1.70158; let q = p - 1.0; 1.0 + (c + 1.0) * q * q * q + c * q * q }
        Easing::EaseIn => p * p,
        Easing::EaseOut => p * (2.0 - p),
        Easing::EaseInOut => if p < 0.5 { 2.0 * p * p } else { 1.0 - 2.0 * (1.0 - p) * (1.0 - p) },
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::eval(x1, y1, x2, y2, p),
    }
}

/// The camera panel's un-overridden (layout-resolved) rect as a `CamPose` - `CameraMoveTrack::
/// sample`'s `live` argument, which its entry blend eases FROM and its exit blend eases back TO.
/// Recomputed every frame from that frame's own scene, so a layout transition still in flight
/// moves it. `ow`/`oh` are the output frame's pixel dims (same basis `rect_from_center` converts
/// back into); inverse of that conversion (center + height fraction, not top-left rect).
pub fn static_cam_pose(rect: RectF, ow: f32, oh: f32) -> CamPose {
    CamPose { x: (rect.x + rect.w * 0.5) / ow, y: (rect.y + rect.h * 0.5) / oh, size: rect.h / oh }
}

/// Shrink `(zi, zo)` proportionally so `zi + zo <= span`, keeping both ramps inside the pill.
/// `pub(crate)` because layout segments need the same rule (`scene::layout`): an entry that
/// outlasts its own segment never settles, yet hands that unreached scene to whatever blends off
/// it next. One definition, so the two kinds of region cannot fit their ramps differently.
pub(crate) fn fit_durations(zi: u32, zo: u32, span: u32) -> (u32, u32) {
    let total = zi + zo;
    if total <= span || total == 0 { return (zi, zo); }
    let f = span as f32 / total as f32;
    ((zi as f32 * f) as u32, (zo as f32 * f) as u32)
}

/// One in-flight handoff between whichever region was previously "in control" and the
/// new winner - eases the camera from wherever it actually is (`from_*`) toward the new
/// winner's own natural target over `dur_ms`, using the entering (or exiting) region's
/// own easing curve. See `CameraSim::step`.
struct Transition { from_scale: f32, from_cx: f32, from_cy: f32, start_ms: u32, dur_ms: u32, easing: Easing }

/// A virtual camera whose zoom scale follows a deterministic eased curve contained
/// entirely within each zoom region: it ramps 1 -> target over `zoom_in_ms`, holds,
/// then ramps target -> 1 over `zoom_out_ms`, reaching 1 exactly at `end_ms` (so the
/// timeline pill is an honest bound). The center eases in lockstep on zoom-in toward the
/// aim - `r.anchor`, or the LIVE cursor for a `follow_cursor` region - pans to keep the
/// cursor in view during hold, and is recentred by the in-frame clamp as scale returns to 1.
///
/// The **highest-`layer`** active region wins an overlap (ties broken by the most
/// recently added, i.e. the later index) - not simply whichever is most recent. When the
/// winner changes, `transition` eases the output from wherever the camera currently is to
/// the new winner's own target, instead of assuming a fresh start from frame-center/scale-1
/// (that assumption is only valid, and still used, for a genuine `None -> Some` fresh start).
pub struct CameraSim {
    frame_w: u32, frame_h: u32, cx: f32, cy: f32, scale: f32,
    driver: Option<usize>,
    // The current driver's own zoom_out_ms/easing, refreshed every step it's the driver - so a
    // handoff to `None` can build its exit transition from these without re-indexing into the
    // CURRENT call's `regions`, which may be a different/shorter slice than the one that was
    // passed when `driver` was captured (e.g. regions rebuilt after an edit, or - as one existing
    // test exercises directly - a caller passing `&[]` once a region's span has fully elapsed).
    driver_zoom_out_ms: u32,
    driver_easing: Easing,
    transition: Option<Transition>,
}

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self { frame_w, frame_h, cx: frame_w as f32 / 2.0, cy: frame_h as f32 / 2.0, scale: 1.0,
            driver: None, driver_zoom_out_ms: 450, driver_easing: Easing::Smooth, transition: None }
    }

    /// The highest-layer region active at `t_ms`, as its index into `regions` - ties
    /// (equal layer) broken by the larger index (the most recently added).
    fn winner(regions: &[ZoomRegion], t_ms: u32) -> Option<usize> {
        regions.iter().enumerate()
            .filter(|(_, r)| t_ms >= r.start_ms && t_ms <= r.end_ms)
            .max_by_key(|(i, r)| (r.layer, *i))
            .map(|(i, _)| i)
    }

    /// How much of `r`'s zoom-in window is still ahead of `t_ms` (its full `zoom_in_ms`
    /// once that window has already elapsed) - the length a handoff blend INTO `r` must
    /// run so it finishes exactly where `r`'s own ramp does.
    fn remaining_zoom_in(r: &ZoomRegion, t_ms: u32) -> u32 {
        let span = r.end_ms.saturating_sub(r.start_ms).max(1);
        let (zi, _) = fit_durations(r.zoom_in_ms, r.zoom_out_ms, span);
        let zin_end = r.start_ms + zi;
        if t_ms < zin_end { zin_end - t_ms } else { r.zoom_in_ms }
    }

    pub fn step(&mut self, t_ms: u32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera {
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        let winner_idx = Self::winner(regions, t_ms);

        if winner_idx != self.driver {
            if self.driver.is_some() {
                // A real handoff: something -> something else, or something -> nothing.
                // Ease from wherever the camera actually is right now, over what is LEFT of
                // the incoming region's zoom-in window, so the blend lands exactly where its
                // own ramp would have (no second curve running past the ramp's end).
                let (dur_ms, easing) = match winner_idx {
                    Some(i) => (Self::remaining_zoom_in(&regions[i], t_ms), regions[i].easing),
                    None => (self.driver_zoom_out_ms, self.driver_easing),
                };
                self.transition = Some(Transition {
                    from_scale: self.scale, from_cx: self.cx, from_cy: self.cy,
                    start_ms: t_ms, dur_ms: dur_ms.max(1), easing,
                });
            } else {
                // `None -> Some` (a fresh start) needs no transition - the natural zoom-in-
                // from-center computation below is already correct. Any transition still in
                // flight here is the previous driver's stale `Some -> None` exit blend, which
                // would otherwise attenuate this region's whole ramp; drop it.
                self.transition = None;
            }
            self.driver = winner_idx;
        }
        if let Some(i) = winner_idx {
            self.driver_zoom_out_ms = regions[i].zoom_out_ms;
            self.driver_easing = regions[i].easing;
        }

        // The winner's own natural target for this instant - unchanged per-phase math.
        let (natural_scale, natural_cx, natural_cy) = match winner_idx {
            None => (1.0, fw / 2.0, fh / 2.0),
            Some(i) => {
                let r = &regions[i];
                let span = r.end_ms.saturating_sub(r.start_ms).max(1);
                let (zi, zo) = fit_durations(r.zoom_in_ms, r.zoom_out_ms, span);
                let (zin_end, zout_start) = (r.start_ms + zi, r.end_ms.saturating_sub(zo));
                let s = r.target_scale;
                if t_ms < zin_end {
                    // While a handoff blend is in flight the natural target is this region's
                    // STEADY pose, not its own ramp: easing both on the same clock multiplies
                    // the two curves and pulses the camera back OUT mid-handoff.
                    let e = if self.transition.is_some() { 1.0 }
                        else { ease(r.easing, (t_ms - r.start_ms) as f32 / zi.max(1) as f32) };
                    // A cursor-target zoom re-aims at the LIVE cursor every step, so the ramp
                    // ends where the follow phase would already be (no second, visible move).
                    let aim = if r.follow_cursor { cursor } else { r.anchor };
                    (1.0 + (s - 1.0) * e,
                     fw / 2.0 + (aim.x as f32 - fw / 2.0) * e,
                     fh / 2.0 + (aim.y as f32 - fh / 2.0) * e)
                } else if t_ms >= zout_start {
                    let e = ease(r.easing, (r.end_ms - t_ms) as f32 / zo.max(1) as f32);
                    (1.0 + (s - 1.0) * e, self.cx, self.cy)
                } else {
                    let (mx, my) = (fw / (2.0 * s) * 0.4, fh / (2.0 * s) * 0.4);
                    let (dx, dy) = (cursor.x as f32 - self.cx, cursor.y as f32 - self.cy);
                    let tx = if dx > mx { cursor.x as f32 - mx } else if dx < -mx { cursor.x as f32 + mx } else { self.cx };
                    let ty = if dy > my { cursor.y as f32 - my } else if dy < -my { cursor.y as f32 + my } else { self.cy };
                    let k = cfg.follow_damping.clamp(0.0, 1.0);
                    (s, self.cx + (tx - self.cx) * k, self.cy + (ty - self.cy) * k)
                }
            }
        };

        // Blend toward the natural target while a handoff transition is in flight; the
        // blended value feeds back into self.{scale,cx,cy} so next step's hold-phase
        // cursor-follow damps from the blended position, not the raw one.
        if let Some(tr) = &self.transition {
            let elapsed = t_ms.saturating_sub(tr.start_ms);
            if elapsed < tr.dur_ms {
                let e = ease(tr.easing, elapsed as f32 / tr.dur_ms as f32);
                self.scale = tr.from_scale + (natural_scale - tr.from_scale) * e;
                self.cx = tr.from_cx + (natural_cx - tr.from_cx) * e;
                self.cy = tr.from_cy + (natural_cy - tr.from_cy) * e;
            } else {
                self.transition = None;
                self.scale = natural_scale; self.cx = natural_cx; self.cy = natural_cy;
            }
        } else {
            self.scale = natural_scale; self.cx = natural_cx; self.cy = natural_cy;
        }

        let (half_w, half_h) = (fw / (2.0 * self.scale.max(0.01)), fh / (2.0 * self.scale.max(0.01)));
        self.cx = self.cx.clamp(half_w, (fw - half_w).max(half_w));
        self.cy = self.cy.clamp(half_h, (fh - half_h).max(half_h));
        Camera { cx: self.cx, cy: self.cy, scale: self.scale }
    }
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod cursor_tests;

pub mod autozoom;
pub mod manual;
pub mod moves;
