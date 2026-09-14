use crate::export::camera::moves::CamPose;
use crate::export::scene::Panel;
use crate::export::types::{Camera, Easing, FramePoint, ZoomConfig, ZoomRegion};

/// Normalized easing curve `[0,1] -> [0,1]`. `Smooth` = smoothstep; `Linear` = identity;
/// `Spring` = ease-out-back (a small overshoot past 1 near the end, then settle);
/// `EaseIn`/`EaseOut`/`EaseInOut` = quadratic accelerate / decelerate / both. `pub(crate)`
/// so `fx_state`'s `SpotlightSim` can reuse the same curves for its own handoff transitions.
pub(crate) fn ease(e: Easing, p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    match e {
        Easing::Linear => p,
        Easing::Smooth => p * p * (3.0 - 2.0 * p),
        Easing::Spring { .. } => crate::export::spring::eval(e, p),
        Easing::EaseIn => p * p,
        Easing::EaseOut => p * (2.0 - p),
        Easing::EaseInOut => if p < 0.5 { 2.0 * p * p } else { 1.0 - 2.0 * (1.0 - p) * (1.0 - p) },
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::eval(x1, y1, x2, y2, p),
    }
}

/// The un-overridden (layout-resolved) camera panel as a `CamPose` - `CameraMoveTrack::sample`'s
/// `live` argument: what its blends ease from/to and what a shape-inheriting keyframe's `round`
/// is. Recomputed every frame, so a layout transition still in flight moves it. The inverse of
/// `rect_from_center` + `override_camera` (center + height fraction + corner fraction of the short side).
pub fn static_cam_pose(panel: &Panel, ow: f32, oh: f32) -> CamPose {
    let r = panel.rect;
    CamPose { x: (r.x + r.w * 0.5) / ow, y: (r.y + r.h * 0.5) / oh, size: r.h / oh,
              round: Some(panel.radius / r.w.min(r.h).max(0.001)) }
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

/// A virtual camera whose zoom scale follows a deterministic eased curve contained
/// entirely within each zoom region: it ramps 1 -> target over `zoom_in_ms`, holds,
/// then ramps target -> 1 over `zoom_out_ms`, reaching 1 exactly at `end_ms` (so the
/// timeline pill is an honest bound). The center eases in lockstep on zoom-in toward the
/// region's aim (`follow::aim` - one point shared by every phase), damps toward that same
/// aim during hold, and is recentred by the in-frame clamp as scale returns to 1.
///
/// The **highest-`layer`** active region wins an overlap (ties broken by the most
/// recently added, i.e. the later index) - not simply whichever is most recent. When the
/// winner changes, `transition` (`handoff.rs`) eases the output from wherever the camera
/// currently is - and at whatever velocity it currently has - to the new winner's own target,
/// instead of assuming a fresh start from frame-center/scale-1 at a standstill (that assumption
/// is only valid, and still used, for a genuine `None -> Some` fresh start).
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
    transition: Option<handoff::Transition>,
    // The `(cx, cy, scale)` velocity per MILLISECOND over the previous step - what a handoff blend
    // carries across the seam so a moving camera is not stopped dead (H2) - and whether there WAS
    // a previous step to measure it over.
    vel: (f32, f32, f32),
    stepped: bool,
    // Opt-in output smoothing (`ZoomConfig::smoothing_ms`); a no-op, and never primed, at 0.
    smooth: smoothing::Damped2,
}

impl CameraSim {
    pub fn new(frame_w: u32, frame_h: u32) -> Self {
        Self { frame_w, frame_h, cx: frame_w as f32 / 2.0, cy: frame_h as f32 / 2.0, scale: 1.0,
            driver: None, driver_zoom_out_ms: 450, driver_easing: Easing::Smooth, transition: None,
            vel: (0.0, 0.0, 0.0), stepped: false, smooth: smoothing::Damped2::new() }
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

    pub fn step(&mut self, t_ms: u32, dt_ms: f32, cursor: FramePoint, regions: &[ZoomRegion], cfg: &ZoomConfig) -> Camera {
        let (fw, fh) = (self.frame_w as f32, self.frame_h as f32);
        let prev = Camera { cx: self.cx, cy: self.cy, scale: self.scale };
        // The caller's EXACT step length (`1000 / out_fps`, never rounded) - `t_ms` is truncated to
        // whole milliseconds, so differencing it would read 16/17/17/16 on a 60fps grid and put a
        // ~3%/frame ripple on every rate below. Guarded only against 0/NaN.
        let dt = if dt_ms.is_finite() { dt_ms.clamp(0.1, 1000.0) } else { follow::REF_STEP_MS };
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
                self.transition = Some(handoff::Transition::open(prev, self.vel, dt, dur_ms as f32, easing));
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
                // ONE aim for every phase (`follow::aim`): the ramp eases toward exactly the
                // point the hold then damps toward, so nothing moves at the boundary.
                let (tx, ty) = follow::aim(r, cursor);
                if t_ms < zin_end {
                    // While a handoff blend is in flight the natural target is this region's
                    // STEADY pose, not its own ramp: easing both on the same clock multiplies
                    // the two curves and pulses the camera back OUT mid-handoff.
                    let e = if self.transition.is_some() { 1.0 }
                        else { ease(r.easing, (t_ms - r.start_ms) as f32 / zi.max(1) as f32) };
                    (1.0 + (s - 1.0) * e, fw / 2.0 + (tx - fw / 2.0) * e, fh / 2.0 + (ty - fh / 2.0) * e)
                } else {
                    // Hold AND zoom-out share the centre: only the SCALE differs, easing back to
                    // 1 over `zo`. Freezing the centre for the ramp-out stopped a moving follow
                    // dead and turned a clamped pose's release into a large reverse sweep (H5).
                    let scale = if t_ms >= zout_start {
                        1.0 + (s - 1.0) * ease(r.easing, (r.end_ms - t_ms) as f32 / zo.max(1) as f32)
                    } else { s };
                    let k = follow::damping(cfg.follow_damping, dt);
                    (scale, self.cx + (tx - self.cx) * k, self.cy + (ty - self.cy) * k)
                }
            }
        };

        // Blend toward the natural target while a handoff transition is in flight; the
        // blended value feeds back into self.{scale,cx,cy} so next step's hold-phase
        // cursor-follow damps from the blended position, not the raw one.
        let natural = Camera { cx: natural_cx, cy: natural_cy, scale: natural_scale };
        let out = match self.transition.as_mut().and_then(|tr| tr.blend(dt, natural)) {
            Some(blended) => blended,
            None => { self.transition = None; natural }
        };
        // Floor at full frame: a handoff's velocity carry (`h10 * vel`) can push a zoom-out that
        // ends at speed (Linear, Spring) below 1, which draws the frame smaller than the output.
        (self.scale, self.cx, self.cy) = (out.scale.max(1.0), out.cx, out.cy);

        let (half_w, half_h) = (fw / (2.0 * self.scale.max(0.01)), fh / (2.0 * self.scale.max(0.01)));
        self.cx = self.cx.clamp(half_w, (fw - half_w).max(half_w));
        self.cy = self.cy.clamp(half_h, (fh - half_h).max(half_h));
        // Velocity of the step just taken, for the next handoff. Zero on the very first step:
        // `prev` is then the constructor's frame-centre pose, not a place the camera ever was.
        self.vel = if self.stepped {
            ((self.cx - prev.cx) / dt, (self.cy - prev.cy) / dt, (self.scale - prev.scale) / dt)
        } else { (0.0, 0.0, 0.0) };
        self.stepped = true;
        let raw = Camera { cx: self.cx, cy: self.cy, scale: self.scale };
        self.smooth.apply(dt, raw, cfg.smoothing_ms, fw, fh) // re-clamps; identity at 0
    }
}

#[cfg(test)] #[path = "camera_tests.rs"] mod tests;
#[cfg(test)] #[path = "cursor_tests.rs"] mod cursor_tests;
#[cfg(test)] #[path = "jank_probe_tests.rs"] mod jank_probe_tests;

// `follow` is `pub(crate)` for ONE reason: `export::cursor`'s low-pass reads `damping`/
// `REF_STEP_MS`, so the camera and the cursor cannot drift apart on what a setting means.
pub mod autozoom; pub(crate) mod follow; mod handoff; pub mod manual; pub mod moves; mod smoothing;
