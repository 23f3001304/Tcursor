use super::*;

#[test]
fn the_bundled_face_loads() {
    assert!(
        font().is_some(),
        "assets/fonts/Inter-SemiBold.ttf must parse"
    );
}

#[test]
fn run_width_grows_with_the_text_and_with_the_size() {
    let f = font().unwrap();
    let a = run_width(&f, 32.0, "Hello");
    let b = run_width(&f, 32.0, "Hello world");
    let c = run_width(&f, 64.0, "Hello");
    assert!(b > a && c > a * 1.9, "{a} {b} {c}");
    assert_eq!(run_width(&f, 32.0, ""), 0.0);
}

#[test]
fn draw_run_paints_at_the_left_edge_and_the_baseline() {
    let (w, h) = (200u32, 80u32);
    let f = font().unwrap();
    let mut out = vec![0u8; (w * h * 4) as usize];
    draw_run(
        &mut out,
        w,
        h,
        &f,
        40.0,
        20.0,
        60.0,
        "IJ",
        [255, 255, 255],
        1.0,
        false,
    );
    let lit: Vec<(u32, u32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|(x, y)| out[((y * w + x) * 4) as usize] > 40)
        .collect();
    assert!(!lit.is_empty(), "the run painted nothing");
    let minx = lit.iter().map(|p| p.0).min().unwrap();
    let maxy = lit.iter().map(|p| p.1).max().unwrap();
    assert!(minx >= 18 && minx <= 30, "left edge near x=20, got {minx}");
    assert!(
        maxy <= 62,
        "nothing below the baseline for these glyphs, got {maxy}"
    );
}

#[test]
fn a_shadow_darkens_one_pixel_down_and_right_and_is_optional() {
    let (w, h) = (120u32, 60u32);
    let f = font().unwrap();
    let mut with = vec![160u8; (w * h * 4) as usize];
    let mut without = with.clone();
    draw_run(
        &mut with,
        w,
        h,
        &f,
        30.0,
        10.0,
        40.0,
        "M",
        [255, 255, 255],
        1.0,
        true,
    );
    draw_run(
        &mut without,
        w,
        h,
        &f,
        30.0,
        10.0,
        40.0,
        "M",
        [255, 255, 255],
        1.0,
        false,
    );
    assert_ne!(with, without, "the shadow must change some pixel");
    assert!(
        with.iter().zip(&without).any(|(a, b)| a < b),
        "and it must darken rather than lighten"
    );
}

#[test]
fn put_blends_over_bgra_and_ignores_out_of_bounds() {
    let (w, h) = (4u32, 4u32);
    let mut out = vec![0u8; (w * h * 4) as usize];
    put(&mut out, w, h, 1, 1, [255, 0, 0], 1.0);
    let i = ((1 * w + 1) * 4) as usize;
    assert_eq!(
        [out[i], out[i + 1], out[i + 2]],
        [0, 0, 255],
        "red lands in the R byte, index 2"
    );
    put(&mut out, w, h, -1, 0, [255, 255, 255], 1.0);
    put(&mut out, w, h, 9, 9, [255, 255, 255], 1.0);
    put(&mut out, w, h, 0, 0, [255, 255, 255], 0.0);
    assert_eq!(out[0], 0, "alpha zero and out of bounds are no-ops");
}
