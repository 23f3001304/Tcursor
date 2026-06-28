use crate::export::fx_state::VideoFx;
use crate::settings::model::VideoFxMode;

/// Apply the full-frame video effect onto the composited BGRA frame.
pub fn draw_video(out: &mut [u8], ow: u32, oh: u32, v: &VideoFx) {
    let a = v.alpha.clamp(0.0, 1.0);
    if a <= 0.0 { return; }
    let (fcx, fcy) = (ow as f32 / 2.0, oh as f32 / 2.0);
    let maxd = (fcx * fcx + fcy * fcy).sqrt().max(1.0);
    let (mx, my) = (ow as f32 * 0.08, oh as f32 * 0.08);
    for y in 0..oh {
        for x in 0..ow {
            let i = ((y * ow + x) * 4) as usize;
            match v.mode {
                VideoFxMode::CinematicDim => {
                    let vg = ((x as f32 - fcx).hypot(y as f32 - fcy) / maxd).clamp(0.0, 1.0);
                    let k = 1.0 - a * (0.2 + 0.5 * vg);
                    for c in 0..3 { out[i + c] = (out[i + c] as f32 * k).round() as u8; }
                }
                VideoFxMode::ScreenFocus => {
                    let inside = (x as f32) > mx && (y as f32) > my && (x as f32) < ow as f32 - mx && (y as f32) < oh as f32 - my;
                    if !inside { let k = 1.0 - a * 0.6; for c in 0..3 { out[i + c] = (out[i + c] as f32 * k).round() as u8; } }
                }
                VideoFxMode::ColorPop => {
                    let l = 0.299 * out[i + 2] as f32 + 0.587 * out[i + 1] as f32 + 0.114 * out[i] as f32;
                    for c in 0..3 { let p = (out[i + c] as f32 - l) * 1.6 + l; out[i + c] = (out[i + c] as f32 * (1.0 - a) + p.clamp(0.0, 255.0) * a).round() as u8; }
                }
                VideoFxMode::NebulaWash => {
                    // CPU fallback: a flat tint wash (no per-pixel fbm at 4K).
                    let tint = [120u8, 90, 255];
                    for c in 0..3 { out[i + c] = (out[i + c] as f32 * (1.0 - a * 0.4) + tint[2 - c] as f32 * a * 0.4).round() as u8; }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::VideoFxMode;
    fn v(mode: VideoFxMode) -> VideoFx { VideoFx { mode, alpha: 1.0, t: 0.0 } }
    #[test]
    fn cinematic_darkens_corner() {
        let (w,h)=(100u32,100u32); let mut out = vec![200u8; (w*h*4) as usize];
        draw_video(&mut out, w, h, &v(VideoFxMode::CinematicDim));
        assert!(out[0] < 200, "cinematic dims the frame");
    }
    #[test]
    fn focus_dims_edge_not_center() {
        let (w,h)=(100u32,100u32); let mut out = vec![200u8; (w*h*4) as usize];
        draw_video(&mut out, w, h, &v(VideoFxMode::ScreenFocus));
        assert!(out[0] < out[((50*w+50)*4) as usize], "focus dims the edge vs center");
    }
}
