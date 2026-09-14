use super::*;
fn spr() -> CursorSprite { CursorSprite { bgra: vec![255u8; 4 * 4 * 4], w: 4, h: 4, hot: (0.0, 0.0), canvas_h: 4 } }

#[test]
fn bounce_is_identity_when_disabled_or_idle() {
    assert_eq!(bounce_scale(&[], 1000, true, 0.5), 1.0);
    assert_eq!(bounce_scale(&[500], 1000, false, 0.5), 1.0); // disabled
    assert_eq!(bounce_scale(&[100], 5000, true, 0.5), 1.0);  // click long past -> recovered
}
#[test]
fn bounce_dips_right_after_a_click() {
    let s = bounce_scale(&[1000], 1010, true, 0.5); // 10ms after a click
    assert!(s < 1.0 && s > 0.5, "dips below 1.0 just after a click, got {s}");
}
#[test]
fn higher_intensity_produces_deeper_dip() {
    let deep = bounce_scale(&[1000], 1000, true, 1.0);
    let shallow = bounce_scale(&[1000], 1000, true, 0.3);
    assert!(deep < shallow, "intensity 1.0 should dip deeper than 0.3, got {deep} vs {shallow}");
}
#[test]
fn draws_pixels_at_the_position() {
    let (w, h) = (40u32, 40u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    draw_cursor(&mut out, w, h, &spr(), (20.0, 20.0), &[], 8.0, 0.0, 1.0, (0, 0, w as i32, h as i32));
    assert!(out.iter().any(|&b| b > 0), "cursor blit wrote visible pixels");
}
/// The ghost-cursor mechanism `FrameRenderer::reset_camera` now clears: a stale `recent` entry
/// (a PREVIOUS scrub target) blits a faded sprite THERE. Clearing it removes the ghost.
#[test]
fn a_stale_trail_point_draws_a_ghost_until_recent_is_cleared() {
    let mut recent = std::collections::VecDeque::from(vec![(4.0f32, 4.0f32)]);
    let ghost_alpha = |r: &mut std::collections::VecDeque<(f32, f32)>| {
        let mut out = vec![0u8; 40 * 40 * 4];
        apply_enhanced(&mut out, 40, 40, &spr(), (30.0, 30.0), r, 6, &[], 0, 1.0, 0.9, false, 0.5, 1.0, (0, 0, 40, 40), BusyPose::still(), 1.0);
        out[(4 * 40 + 4) * 4 + 3]
    };
    assert!(ghost_alpha(&mut recent) > 0, "a stale trail point paints a ghost cursor at (4,4)");
    recent.clear();
    assert_eq!(ghost_alpha(&mut recent), 0, "a cleared trail leaves no ghost");
}
#[test]
fn offscreen_position_is_safe_noop() {
    let (w, h) = (40u32, 40u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    draw_cursor(&mut out, w, h, &spr(), (1000.0, 1000.0), &[], 8.0, 0.0, 1.0, (0, 0, w as i32, h as i32)); // must not panic / OOB
    assert!(out.iter().all(|&b| b == 0));
}

/// A 2x2 sprite of pure black and pure white, with a hotspot at its top-left.
fn checker() -> CursorSprite {
    let bgra = vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255];
    CursorSprite { bgra, w: 2, h: 2, hot: (0.0, 0.0), canvas_h: 2 }
}

/// THE FAST-PATH PIN. A still pose must take `blit`'s nearest-neighbour copy, not `cursorxform`'s
/// bilinear one - so at a fractional scale every painted pixel keeps one of the sprite's ORIGINAL
/// channel values. Bilinear sampling would produce in-between greys, which is exactly what must
/// not happen to the ordinary (non-busy) cursor now that a transformed path exists.
#[test]
fn a_still_pose_keeps_the_nearest_neighbour_blit_and_never_interpolates() {
    let mut out = vec![0u8; 40 * 40 * 4];
    draw_cursor_posed(&mut out, 40, 40, &checker(), (10.0, 10.0), &[], 3.4, 0.0, 1.0,
        (0, 0, 40, 40), BusyPose::still(), 1.0);
    let painted: Vec<u8> = out.chunks_exact(4).filter(|p| p[3] > 0).map(|p| p[0]).collect();
    assert!(!painted.is_empty(), "the still cursor drew something");
    assert!(painted.iter().all(|&v| v == 0 || v == 255),
        "a still pose must not interpolate; found {:?}", painted.iter().find(|&&v| v != 0 && v != 255));
    // And `draw_cursor` (no pose argument at all) is exactly that still path.
    let mut plain = vec![0u8; 40 * 40 * 4];
    draw_cursor(&mut plain, 40, 40, &checker(), (10.0, 10.0), &[], 3.4, 0.0, 1.0, (0, 0, 40, 40));
    assert_eq!(plain, out, "draw_cursor must stay byte-identical to the still-posed draw");
}

/// The other half: a real pose DOES reach the transformed blit, so the pin above is meaningful.
#[test]
fn a_rotated_pose_reaches_the_transformed_blit() {
    let render = |pose: BusyPose| {
        let mut out = vec![0u8; 40 * 40 * 4];
        draw_cursor_posed(&mut out, 40, 40, &checker(), (10.0, 10.0), &[], 8.0, 0.0, 1.0,
            (0, 0, 40, 40), pose, 1.0);
        out
    };
    assert_ne!(render(BusyPose { frame: 0, angle_deg: 90.0, scale: 1.0 }), render(BusyPose::still()));
    assert_ne!(render(BusyPose { frame: 0, angle_deg: 0.0, scale: 1.06 }), render(BusyPose::still()));
}
