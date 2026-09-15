use super::*;

#[test]
fn blit_clips_a_negative_origin_instead_of_snapping_to_zero() {
    let (dw, dh) = (400u32, 1u32);
    let (sw, sh) = (200u32, 1u32);
    let mut src = vec![0u8; (sw * sh * 4) as usize];
    for x in 0..sw {
        let i = (x * 4) as usize;
        src[i] = (x % 256) as u8;
        src[i + 3] = 255;
    }
    let mut dst = vec![9u8; (dw * dh * 4) as usize];
    blit(&mut dst, dw, dh, &src, sw, sh, -50, 0, None, |_, _| 1.0);
    assert_eq!(
        dst[0], 50,
        "dst col 0 should show source col 50 (the clipped visible sub-rect)"
    );
    assert_eq!(
        dst[(149 * 4) as usize],
        199,
        "dst col 149 (panel's right edge - 1) should show source col 199"
    );
    assert_eq!(
        dst[(150 * 4) as usize],
        9,
        "beyond the panel's true right edge (150) must stay untouched"
    );
}

#[test]
fn blit_ring_clips_a_negative_origin_the_same_way_as_blit() {
    let (dw, dh) = (400u32, 1u32);
    let (pw, ph) = (200u32, 1u32);
    let mut dst = vec![9u8; (dw * dh * 4) as usize];
    blit_ring(
        &mut dst,
        dw,
        dh,
        pw,
        ph,
        -50,
        0,
        0.0,
        200.0,
        [255, 0, 0],
        1.0,
    );
    let col0 = &dst[0..4];
    assert!(
        col0[2] > col0[0] && col0[2] > 100,
        "dst col 0 should be strongly ring-painted red, got {:?}",
        col0
    );
    assert_eq!(&dst[(150 * 4) as usize..(150 * 4 + 4) as usize], &[9, 9, 9, 9],
        "beyond the panel's true right edge (150) must stay untouched, not ring-painted from a snapped origin");
}

#[test]
fn blit_opaque_inner_skip_is_byte_identical_to_full_sdf() {
    let (pw, ph, r) = (40u32, 40u32, 6.0f32);
    let mut src = vec![0u8; (pw * ph * 4) as usize];
    for (i, px) in src.chunks_mut(4).enumerate() {
        px.copy_from_slice(&[
            (i % 251) as u8,
            (i * 3 % 251) as u8,
            (i * 7 % 251) as u8,
            255,
        ]);
    }
    let (dw, dh, ox, oy) = (48u32, 48u32, 4i32, 4i32);
    let (hw, hh) = (pw as f32 / 2.0, ph as f32 / 2.0);
    let cov = move |tx: u32, ty: u32| {
        let qx = ((tx as f32 + 0.5) - hw).abs() - (hw - r);
        let qy = ((ty as f32 + 0.5) - hh).abs() - (hh - r);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        let d = qx.max(qy).min(0.0) + outside - r;
        (0.5 - d).clamp(0.0, 1.0) * 1.0
    };
    let ins = (r.ceil() as u32) + 2;
    let inner = (pw > 2 * ins && ph > 2 * ins).then(|| (ins, ins, pw - ins, ph - ins));
    assert!(
        inner.is_some(),
        "inner rect must be non-empty for this test to mean anything"
    );
    let base = vec![30u8; (dw * dh * 4) as usize];
    let mut fast = base.clone();
    blit(&mut fast, dw, dh, &src, pw, ph, ox, oy, inner, cov);
    let mut full = base.clone();
    blit(&mut full, dw, dh, &src, pw, ph, ox, oy, None, cov);
    assert_eq!(fast, full, "opaque-inner skip diverged from full SDF");
}
