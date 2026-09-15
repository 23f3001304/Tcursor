use crate::export::fx::fx_state::{FxRenderer, FxState};

pub struct CpuFx;

impl FxRenderer for CpuFx {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState) {
        if let Some(v) = state.video {
            crate::export::fx::videodraw::draw_video(out, ow, oh, &v);
        }
        if let Some(s) = state.spot {
            crate::export::fx::spot::spotdraw::draw_spot(out, ow, oh, &s, state.intensity);
        }
        crate::export::fx::click::clickdraw::draw_clicks(out, ow, oh, state);
        if let Some(l) = &state.lens {
            crate::export::fx::lens::draw::draw_lens(out, ow, oh, l, state.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fx::fx_state::{FxHit, FxRenderer, FxState, Spot};
    use crate::settings::model::{ClickFxStyle, SpotlightMode};

    #[test]
    fn ripple_paints_a_colored_ring() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Ripple,
            color: [255, 0, 0],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 50.0,
                y: 50.0,
                progress: 0.5,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out.chunks(4).any(|p| p[2] > 40),
            "ripple should paint red (R at BGRA idx 2)"
        );
    }
    #[test]
    fn spotlight_dims_corner_more_than_center() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![200u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::None,
            color: [0, 0, 0],
            intensity: 1.0,
            hits: vec![],
            spot: Some(Spot {
                cx: 50.0,
                cy: 50.0,
                dim: 0.6,
                radius_frac: 0.13,
                feather_frac: 0.10,
                alpha: 1.0,
                mode: SpotlightMode::Classic,
                tint: [0, 0, 0],
                t: 0.0,
                cam_rect: [0.0; 4],
                cam_radius: 0.0,
                dim_camera: true,
            }),
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out[0] < out[((50 * w + 50) * 4) as usize],
            "corner dimmer than lit center"
        );
    }
    #[test]
    fn glow_brightens_near_click() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![10u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Glow,
            color: [255, 255, 255],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 50.0,
                y: 50.0,
                progress: 0.2,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out[((50 * w + 50) * 4) as usize] > 10,
            "glow adds light at the center"
        );
    }
    #[test]
    fn neon_paints_a_bright_ring() {
        let (w, h) = (120u32, 120u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Neon,
            color: [0, 128, 255],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 60.0,
                y: 60.0,
                progress: 0.5,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out.chunks(4).any(|p| p[0] > 60),
            "neon ring paints blue (B at idx 0)"
        );
    }
    #[test]
    fn shockwave_paints_an_expanding_ring() {
        let (w, h) = (140u32, 140u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Shockwave,
            color: [255, 255, 255],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 70.0,
                y: 70.0,
                progress: 0.5,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out.chunks(4).any(|p| p[0] > 40),
            "shockwave fallback paints a ring"
        );
    }
    #[test]
    fn pulse_paints_a_disc_with_a_white_core() {
        let (w, h) = (200u32, 200u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Pulse,
            color: [0, 0, 255],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 100.0,
                y: 100.0,
                progress: 0.2,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        let c = ((100 * w + 100) * 4) as usize;
        assert!(
            out[c] > 200 && out[c + 2] > 40,
            "white core over the blue disc at the click"
        );
        let rim = ((100 * w + 104) * 4) as usize;
        assert!(
            out[rim] > out[rim + 2],
            "the rim is the tint (B at idx 0), not the white core"
        );
    }
    #[test]
    fn every_style_flashes_white_at_the_instant_of_the_click() {
        for style in [
            ClickFxStyle::Glow,
            ClickFxStyle::Shockwave,
            ClickFxStyle::Particles,
            ClickFxStyle::Neon,
        ] {
            let (w, h) = (120u32, 120u32);
            let mut out = vec![0u8; (w * h * 4) as usize];
            let st = FxState {
                style,
                color: [0, 0, 0],
                intensity: 1.0,
                hits: vec![FxHit {
                    x: 60.0,
                    y: 60.0,
                    progress: 0.0,
                }],
                spot: None,
                video: None,
                lens: None,
            };
            CpuFx.apply(&mut out, w, h, &st);
            let c = ((60 * w + 60) * 4) as usize;
            assert!(
                out[c] > 200,
                "{style:?}: flash at the click (got {})",
                out[c]
            );
        }
    }
    #[test]
    fn particles_paint_multiple_specks() {
        let (w, h) = (160u32, 160u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Particles,
            color: [255, 255, 255],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 80.0,
                y: 80.0,
                progress: 0.5,
            }],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(
            out.chunks(4).filter(|p| p[0] > 40).count() > 3,
            "several spark pixels"
        );
    }
    #[test]
    fn empty_state_leaves_frame_untouched() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![7u8; (w * h * 4) as usize];
        let st = FxState {
            style: ClickFxStyle::Ripple,
            color: [255, 0, 0],
            intensity: 1.0,
            hits: vec![],
            spot: None,
            video: None,
            lens: None,
        };
        CpuFx.apply(&mut out, w, h, &st);
        assert!(out.iter().all(|&b| b == 7), "no hits/spot -> no draw");
    }
}
