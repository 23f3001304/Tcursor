use crate::export::fx::fx_state::Spot;
use crate::settings::model::SpotlightMode;

/// Render the cursor spotlight (mode-dependent) onto the BGRA frame.
pub fn draw_spot(out: &mut [u8], ow: u32, oh: u32, s: &Spot) {
    let dim = s.dim.clamp(0.0, 1.0) * s.alpha.clamp(0.0, 1.0);
    if dim <= 0.0 { return; }
    let breathe = if s.mode == SpotlightMode::Breathing { 1.0 + 0.12 * (s.t * 3.1416).sin() } else { 1.0 };
    let r_in = oh as f32 * s.radius_frac.max(0.0) * breathe;
    let r_out = r_in + oh as f32 * s.feather_frac.max(0.001) * breathe;
    let (fcx, fcy) = (ow as f32 / 2.0, oh as f32 / 2.0);
    let maxd = (fcx * fcx + fcy * fcy).sqrt();
    let halo_w = oh as f32 * 0.02;
    for y in 0..oh {
        for x in 0..ow {
            let i = ((y * ow + x) * 4) as usize;
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
                let band = (1.0 - (d - r_in).abs() / halo_w.max(1.0)).clamp(0.0, 1.0);
                if band > 0.0 { add_tint(out, i, s.tint, band); }
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
        Spot { cx: 50.0, cy: 50.0, dim: 0.6, radius_frac: 0.13, feather_frac: 0.10, alpha: 1.0, mode, tint: [120, 60, 255], t: 0.0 }
    }
    #[test]
    fn vignette_dims_corner_not_center() {
        let (w,h)=(100u32,100u32); let mut out = vec![200u8; (w*h*4) as usize];
        draw_spot(&mut out, w, h, &spot(SpotlightMode::Vignette));
        assert!(out[0] < out[((50*w+50)*4) as usize], "vignette darkens corners vs center");
    }
    #[test]
    fn halo_tints_the_ring_edge() {
        let (w,h)=(100u32,100u32); let mut out = vec![10u8; (w*h*4) as usize];
        draw_spot(&mut out, w, h, &spot(SpotlightMode::Halo));
        assert!(out.chunks(4).any(|p| p[0] > 40), "halo paints blue tint (B idx 0)");
    }
}
