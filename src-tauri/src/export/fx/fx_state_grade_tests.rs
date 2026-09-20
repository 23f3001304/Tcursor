use super::*;

#[test]
fn a_grade_runs_with_click_animations_off_and_no_spotlight() {
    use crate::export::grade::params_of;
    use crate::settings::grade::{GradePreset, GradeSettings};
    let (w, h) = (32u32, 32u32);
    let mut out = vec![180u8; (w * h * 4) as usize];
    let mut settings = fx(ClickFxStyle::Ripple, false);
    settings.enabled = false;
    let (exposure, contrast, vignette) = crate::export::grade::seed_of(GradePreset::Noir);
    let g = params_of(&GradeSettings {
        preset: GradePreset::Noir,
        exposure,
        contrast,
        vignette,
    });
    render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &settings,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut SpotlightSim::new(),
        None,
        Vec::new(),
        g,
    );
    let mid = ((16 * w + 16) * 4) as usize;
    assert_ne!(
        out[mid], 180,
        "the grade ran with every click effect switched off"
    );
    assert!(out[0] < out[mid], "and its vignette darkened the corner");
}

#[test]
fn no_grade_and_nothing_else_leaves_the_frame_alone() {
    let (w, h) = (16u32, 16u32);
    let mut out = vec![77u8; (w * h * 4) as usize];
    let mut settings = fx(ClickFxStyle::Ripple, false);
    settings.enabled = false;
    render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &settings,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut SpotlightSim::new(),
        None,
        Vec::new(),
        None,
    );
    assert!(
        out.iter().all(|&b| b == 77),
        "an ungraded project with no effects costs nothing"
    );
}
