use super::*;

#[test]
fn a_mask_survives_click_animations_being_switched_off() {
    use crate::export::fx::fx_masks::MaskDraw;
    let (w, h) = (80u32, 60u32);
    let mut out = vec![200u8; (w * h * 4) as usize];
    let mut fx = crate::settings::model::ClickFxSettings::default();
    fx.enabled = false;
    fx.spotlight = false;
    let masks = vec![MaskDraw {
        mn: [10.0, 10.0],
        mx: [40.0, 40.0],
        r: 0.0,
        feather_px: 1.0,
        amount_px: 4.0,
        dim: 0.6,
        kind: 3,
        alpha: 1.0,
    }];
    crate::export::fx::fx_state::render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &fx,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        crate::export::types::FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut crate::export::fx::fx_state::SpotlightSim::new(),
        None,
        masks,
        None,
    );
    let inside = out[((25 * w + 25) * 4) as usize];
    let outside = out[((55 * w + 5) * 4) as usize];
    assert_eq!(inside, 200, "the highlighted rect keeps its brightness");
    assert!(outside < 120, "everything outside it is dimmed: {outside}");
}
