use crate::export::fx::fx_state::Spot;
use crate::settings::model::SpotlightMode;

/// Rounded-rect coverage: ~1 inside, ~0 outside. Mirrors fx.wgsl's `rrect_cov` exactly.
fn rrect_cov(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32 {
    let (cx, cy) = ((mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5);
    let (hx, hy) = ((mx[0] - mn[0]) * 0.5 - r, (mx[1] - mn[1]) * 0.5 - r);
    let (qx, qy) = ((x - cx).abs() - hx, (y - cy).abs() - hy);
    let sd = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
    (0.5 - sd).clamp(0.0, 1.0)
}

/// Render the cursor spotlight (mode-dependent) onto the BGRA frame. `intensity` is
/// `FxState::intensity`, which fx.wgsl reads as `u.c.z` to scale the Halo ring. When `s.dim_camera`
/// is false, the dim is undone inside `s.cam_rect` (the camera PiP), mirroring fx.wgsl's `camcov`
/// un-dim. Blur/Nebula stay deliberate approximations here (see `factor`): the shader's 4-tap blur
/// needs an unmutated source copy and its nebula needs per-pixel fbm, neither affordable per frame
/// on the CPU - `fx.wgsl` is the reference look and `select_fx` picks it whenever an adapter exists.
pub fn draw_spot(out: &mut [u8], ow: u32, oh: u32, s: &Spot, intensity: f32) {
    let dim = s.dim.clamp(0.0, 1.0) * s.alpha.clamp(0.0, 1.0);
    if dim <= 0.0 { return; }
    let breathe = if s.mode == SpotlightMode::Breathing { 1.0 + 0.12 * (s.t * 3.1416).sin() } else { 1.0 };
    let r_in = oh as f32 * s.radius_frac.max(0.0) * breathe;
    let r_out = r_in + oh as f32 * s.feather_frac.max(0.001) * breathe;
    let (fcx, fcy) = (ow as f32 / 2.0, oh as f32 / 2.0);
    let maxd = (fcx * fcx + fcy * fcy).sqrt();
    let halo_w = oh as f32 * 0.02;
    let inten = intensity.clamp(0.0, 1.0);
    let keep_cam = !s.dim_camera;
    let cam_mn = [s.cam_rect[0], s.cam_rect[1]];
    let cam_mx = [s.cam_rect[2], s.cam_rect[3]];
    for y in 0..oh {
        for x in 0..ow {
            let i = ((y * ow + x) * 4) as usize;
            let pre = [out[i], out[i + 1], out[i + 2]];
            let t = if s.mode == SpotlightMode::Vignette {
                (((x as f32 - fcx).hypot(y as f32 - fcy) / maxd) - 0.4).max(0.0) / 0.6
            } else {
                let d = (x as f32 - s.cx).hypot(y as f32 - s.cy);
                ((d - r_in) / (r_out - r_in)).clamp(0.0, 1.0)
            }.clamp(0.0, 1.0);
            // Blur fallback darkens a touch harder; nebula uses a softer dim + tint wash.
            let factor = if s.mode == SpotlightMode::Nebula { 0.85 } else if s.mode == SpotlightMode::Blur { 1.15 } else { 1.0 };
            let k = (1.0 - (dim * factor).min(1.0) * t).max(0.0);
            if k < 1.0 { for c in 0..3 { out[i + c] = (out[i + c] as f32 * k).round() as u8; } }
            if s.mode == SpotlightMode::Nebula && t > 0.0 {
                add_tint(out, i, s.tint, 0.25 * t);
            }
            if s.mode == SpotlightMode::Halo {
                let d = (x as f32 - s.cx).hypot(y as f32 - s.cy);
                // fx.wgsl scales the halo ring by `u.c.z` (FxState::intensity); without this the
                // CPU ring was always full-strength while the shader's faded with the setting.
                let band = (1.0 - (d - r_in).abs() / halo_w.max(1.0)).clamp(0.0, 1.0);
                if band > 0.0 { add_tint(out, i, s.tint, band * inten); }
            }
            // Undo the dim (and any tint this pixel just picked up) inside the camera rect
            // when the "don't dim the webcam" option is on - matches fx.wgsl's `camcov` mix.
            if keep_cam {
                let cov = rrect_cov(x as f32, y as f32, cam_mn, cam_mx, s.cam_radius);
                if cov > 0.0 {
                    for c in 0..3 {
                        out[i + c] = (out[i + c] as f32 + (pre[c] as f32 - out[i + c] as f32) * cov).round() as u8;
                    }
                }
            }
        }
    }
}

/// Additive tint (BGRA), clamped.
fn add_tint(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    out[i] = (out[i] as f32 + c[2] as f32 * a).min(255.0) as u8;
    out[i + 1] = (out[i + 1] as f32 + c[1] as f32 * a).min(255.0) as u8;
    out[i + 2] = (out[i + 2] as f32 + c[0] as f32 * a).min(255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::SpotlightMode;
    fn spot(mode: SpotlightMode) -> Spot {
        Spot { cx: 50.0, cy: 50.0, dim: 0.6, radius_frac: 0.13, feather_frac: 0.10, alpha: 1.0, mode, tint: [120, 60, 255], t: 0.0,
            cam_rect: [0.0; 4], cam_radius: 0.0, dim_camera: true }
    }
    #[test]
    fn vignette_dims_corner_not_center() {
        let (w,h)=(100u32,100u32); let mut out = vec![200u8; (w*h*4) as usize];
        draw_spot(&mut out, w, h, &spot(SpotlightMode::Vignette), 1.0);
        assert!(out[0] < out[((50*w+50)*4) as usize], "vignette darkens corners vs center");
    }
    #[test]
    fn halo_tints_the_ring_edge() {
        let (w,h)=(100u32,100u32); let mut out = vec![10u8; (w*h*4) as usize];
        draw_spot(&mut out, w, h, &spot(SpotlightMode::Halo), 1.0);
        assert!(out.chunks(4).any(|p| p[0] > 40), "halo paints blue tint (B idx 0)");
    }
    #[test]
    fn halo_ring_scales_with_intensity_like_the_shader() {
        // fx.wgsl: `color + u.tint.rgb * band * u.c.z` - halving FxState::intensity must halve
        // the ring's added tint. Before this fix the CPU ring ignored intensity entirely.
        let (w, h) = (100u32, 100u32);
        let brightest = |inten: f32| {
            let mut out = vec![10u8; (w * h * 4) as usize];
            draw_spot(&mut out, w, h, &spot(SpotlightMode::Halo), inten);
            out.chunks(4).map(|p| p[0]).max().unwrap()
        };
        let (full, half) = (brightest(1.0), brightest(0.5));
        assert!(half < full, "intensity 0.5 must dim the halo ring ({half} vs {full})");
        assert!(brightest(0.0) <= 10, "intensity 0 paints no ring at all");
    }
    #[test]
    fn dim_camera_false_keeps_camera_rect_lit() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![200u8; (w * h * 4) as usize];
        let mut s = spot(SpotlightMode::Classic);
        // Cam rect [0,0,20,20] sits far from the spotlight center (50,50), so its interior
        // would normally dim hard. Probe its center (10,10) - unambiguously inside the rounded
        // rect regardless of corner radius, unlike the bounding-box corner pixel itself (which
        // the rounding legitimately excludes, same as a real rounded-rect SDF).
        s.cam_rect = [0.0, 0.0, 20.0, 20.0]; s.cam_radius = 2.0; s.dim_camera = false;
        draw_spot(&mut out, w, h, &s, 1.0);
        let center_i = ((10 * w + 10) * 4) as usize;
        assert_eq!(out[center_i], 200, "cam-rect center stays at full brightness when dim_camera is false");
    }
    #[test]
    fn dim_camera_true_dims_the_camera_rect_too() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![200u8; (w * h * 4) as usize];
        let mut s = spot(SpotlightMode::Classic);
        s.cam_rect = [0.0, 0.0, 20.0, 20.0]; s.cam_radius = 2.0; s.dim_camera = true;
        draw_spot(&mut out, w, h, &s, 1.0);
        assert!(out[0] < 200, "dim_camera:true -> today's behavior, camera rect dims like everything else");
    }
}
