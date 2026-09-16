use super::*;

fn spot_alpha_channel(
    fx: &dyn FxRenderer,
    w: u32,
    h: u32,
    alpha: f32,
    mode: SpotlightMode,
) -> Vec<u8> {
    let state = FxState {
        spot: Some(Spot {
            cx: w as f32 / 2.0,
            cy: h as f32 / 2.0,
            dim: 0.6,
            radius_frac: 0.13,
            feather_frac: 0.10,
            alpha,
            mode,
            tint: [130, 90, 255],
            t: 0.0,
            cam_rect: [0.0; 4],
            cam_radius: 0.0,
            dim_camera: true,
        }),
        ..Default::default()
    };
    let n = (w * h * 4) as usize;
    let mut on_black = vec![0u8; n];
    for p in on_black.chunks_exact_mut(4) {
        p[3] = 255;
    }
    let mut on_white = vec![255u8; n];
    fx.apply(&mut on_black, w, h, &state);
    fx.apply(&mut on_white, w, h, &state);
    reconstruct(&on_black, &on_white)
        .chunks_exact(4)
        .map(|p| p[3])
        .collect()
}

#[test]
fn classic_spotlight_overlay_alpha_is_proportional_to_spot_alpha() {
    let (w, h) = (48u32, 48u32);
    let (full, half) = with_fx(w, h, |fx| {
        (
            spot_alpha_channel(fx, w, h, 1.0, SpotlightMode::Classic),
            spot_alpha_channel(fx, w, h, 0.5, SpotlightMode::Classic),
        )
    });
    assert!(
        full.iter().any(|&a| a > 40),
        "the spotlight must actually dim something to compare"
    );
    for (i, (&f, &hf)) in full.iter().zip(half.iter()).enumerate() {
        let expect = f as f32 * 0.5;
        assert!(
            (hf as f32 - expect).abs() <= 8.0,
            "px {i}: alpha at 0.5 was {hf}, expected ~{expect} (half of {f})"
        );
    }
}
