use crate::export::fx_state::{FxRenderer, FxState};

/// CPU fallback renderer: draws the spotlight + click rings/discs directly on the
/// composited BGRA buffer. Identical look to the pre-FX-1 overlay (same primitives).
pub struct CpuFx;

impl FxRenderer for CpuFx {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState) {
        if let Some(v) = state.video { crate::export::videodraw::draw_video(out, ow, oh, &v); }
        if let Some(s) = state.spot { crate::export::spotdraw::draw_spot(out, ow, oh, &s); }
        crate::export::clickdraw::draw_clicks(out, ow, oh, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fx_state::{FxHit, FxRenderer, FxState, Spot};
    use crate::settings::model::{ClickFxStyle, SpotlightMode};

    #[test]
    fn ripple_paints_a_colored_ring() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState { style: ClickFxStyle::Ripple, color: [255, 0, 0], intensity: 1.0,
            hits: vec![FxHit { x: 50.0, y: 50.0, progress: 0.5 }], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.chunks(4).any(|p| p[2] > 40), "ripple should paint red (R at BGRA idx 2)");
    }
    #[test]
    fn spotlight_dims_corner_more_than_center() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![200u8; (w * h * 4) as usize];
        let st = FxState { style: ClickFxStyle::None, color: [0, 0, 0], intensity: 1.0, hits: vec![],
            spot: Some(Spot { cx: 50.0, cy: 50.0, dim: 0.6, radius_frac: 0.13, feather_frac: 0.10, alpha: 1.0,
                mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0 }), video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out[0] < out[((50 * w + 50) * 4) as usize], "corner dimmer than lit center");
    }
    #[test]
    fn glow_brightens_near_click() {
        let (w,h)=(100u32,100u32); let mut out = vec![10u8; (w*h*4) as usize];
        let st = FxState { style: ClickFxStyle::Glow, color: [255,255,255], intensity: 1.0,
            hits: vec![FxHit{x:50.0,y:50.0,progress:0.2}], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out[((50*w+50)*4) as usize] > 10, "glow adds light at the center");
    }
    #[test]
    fn neon_paints_a_bright_ring() {
        let (w,h)=(120u32,120u32); let mut out = vec![0u8; (w*h*4) as usize];
        let st = FxState { style: ClickFxStyle::Neon, color: [0,128,255], intensity: 1.0,
            hits: vec![FxHit{x:60.0,y:60.0,progress:0.5}], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.chunks(4).any(|p| p[0] > 60), "neon ring paints blue (B at idx 0)");
    }
    #[test]
    fn shockwave_paints_an_expanding_ring() {
        let (w,h)=(140u32,140u32); let mut out = vec![0u8; (w*h*4) as usize];
        let st = FxState { style: ClickFxStyle::Shockwave, color: [255,255,255], intensity: 1.0,
            hits: vec![FxHit{x:70.0,y:70.0,progress:0.5}], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.chunks(4).any(|p| p[0] > 40), "shockwave fallback paints a ring");
    }
    #[test]
    fn particles_paint_multiple_specks() {
        let (w,h)=(160u32,160u32); let mut out = vec![0u8; (w*h*4) as usize];
        let st = FxState { style: ClickFxStyle::Particles, color: [255,255,255], intensity: 1.0,
            hits: vec![FxHit{x:80.0,y:80.0,progress:0.5}], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.chunks(4).filter(|p| p[0] > 40).count() > 3, "several spark pixels");
    }
    #[test]
    fn empty_state_leaves_frame_untouched() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![7u8; (w * h * 4) as usize];
        let st = FxState { style: ClickFxStyle::Ripple, color: [255,0,0], intensity: 1.0, hits: vec![], spot: None, video: None };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.iter().all(|&b| b == 7), "no hits/spot -> no draw");
    }
}
