use super::*;

const W: u32 = 8;
const H: u32 = 4;

/// A flat nv12 frame: every luma byte `y`, every chroma byte `c`.
fn flat(y: u8, c: u8) -> Vec<u8> {
    let n = (W * H) as usize;
    let mut v = vec![y; n];
    v.extend(std::iter::repeat(c).take(n / 2));
    v
}
fn full() -> RectF { crate::export::coordmap::full_src(W, H) }

/// At alpha 1 the held frame contributes nothing: the blend IS the current frame, byte for byte.
/// This is the state every frame outside a transition would be in, so it must be a pure copy.
#[test]
fn alpha_one_is_the_current_frame_untouched() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 1.0);
    assert_eq!(out, cur);
}

/// At alpha 0 the destination rect is entirely the held frame - and, with matching rects, exactly
/// its own bytes rather than a resampled approximation of them.
#[test]
fn alpha_zero_is_the_held_frame_over_the_destination_rect() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 0.0);
    assert_eq!(out, prev);
}

/// Half way through, every byte of the destination rect is the midpoint of the two pictures - both
/// planes, so a dissolve never shifts colour toward one source.
#[test]
fn half_way_is_the_midpoint_of_both_planes() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 0.5);
    let y_len = (W * H) as usize;
    assert!(out[..y_len].iter().all(|&v| v == 110), "luma midpoint");
    assert!(out[y_len..].iter().all(|&v| v == 150), "chroma midpoint");
}

/// Outside the destination rect nothing is touched: the bars of the NEW span stay exactly as the
/// decoder delivered them, so the crop still has clean edges to cut on.
#[test]
fn pixels_outside_the_destination_rect_are_left_alone() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    // A 4-wide centred band: columns 0..2 and 6..8 are untouched.
    let dst = RectF { x: 2.0, y: 0.0, w: 4.0, h: H as f32 };
    blend_into(&mut out, &cur, &prev, W, H, full(), dst, 0.0);
    for row in 0..H as usize {
        let r = &out[row * W as usize..(row + 1) * W as usize];
        assert_eq!(&r[..2], &[200, 200], "left bar untouched");
        assert_eq!(&r[2..6], &[20, 20, 20, 20], "band blended");
        assert_eq!(&r[6..], &[200, 200], "right bar untouched");
    }
}

/// A ramp resampled from a wide source rect into a narrower destination stays a ramp - the held
/// picture is SCALED into the new span's rect, not re-cropped out of it.
#[test]
fn the_held_picture_is_scaled_into_the_destination_rect() {
    let cur = flat(0, 128);
    let mut prev = vec![0u8; (W * H) as usize];
    for (i, p) in prev.iter_mut().enumerate() { *p = ((i as u32 % W) * 32) as u8; } // 0,32,..,224
    prev.extend(std::iter::repeat(128u8).take((W * H / 2) as usize));
    let mut out = Vec::new();
    let dst = RectF { x: 2.0, y: 0.0, w: 4.0, h: H as f32 };
    blend_into(&mut out, &cur, &prev, W, H, full(), dst, 0.0);
    let band: Vec<u8> = out[2..6].to_vec();
    assert!(band.windows(2).all(|p| p[0] < p[1]), "still increasing left to right: {band:?}");
    assert!(band[0] < 60 && band[3] > 170, "still spans the source's range: {band:?}");
}
