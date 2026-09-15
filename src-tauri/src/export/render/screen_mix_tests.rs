use super::*;

const W: u32 = 8;
const H: u32 = 4;

fn flat(y: u8, c: u8) -> Vec<u8> {
    let n = (W * H) as usize;
    let mut v = vec![y; n];
    v.extend(std::iter::repeat(c).take(n / 2));
    v
}
fn full() -> RectF {
    crate::export::coordmap::full_src(W, H)
}

#[test]
fn alpha_one_is_the_current_frame_untouched() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 1.0);
    assert_eq!(out, cur);
}

#[test]
fn alpha_zero_is_the_held_frame_over_the_destination_rect() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 0.0);
    assert_eq!(out, prev);
}

#[test]
fn half_way_is_the_midpoint_of_both_planes() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    blend_into(&mut out, &cur, &prev, W, H, full(), full(), 0.5);
    let y_len = (W * H) as usize;
    assert!(out[..y_len].iter().all(|&v| v == 110), "luma midpoint");
    assert!(out[y_len..].iter().all(|&v| v == 150), "chroma midpoint");
}

#[test]
fn pixels_outside_the_destination_rect_are_left_alone() {
    let (cur, prev) = (flat(200, 100), flat(20, 200));
    let mut out = Vec::new();
    let dst = RectF {
        x: 2.0,
        y: 0.0,
        w: 4.0,
        h: H as f32,
    };
    blend_into(&mut out, &cur, &prev, W, H, full(), dst, 0.0);
    for row in 0..H as usize {
        let r = &out[row * W as usize..(row + 1) * W as usize];
        assert_eq!(&r[..2], &[200, 200], "left bar untouched");
        assert_eq!(&r[2..6], &[20, 20, 20, 20], "band blended");
        assert_eq!(&r[6..], &[200, 200], "right bar untouched");
    }
}

#[test]
fn the_held_picture_is_scaled_into_the_destination_rect() {
    let cur = flat(0, 128);
    let mut prev = vec![0u8; (W * H) as usize];
    for (i, p) in prev.iter_mut().enumerate() {
        *p = ((i as u32 % W) * 32) as u8;
    }
    prev.extend(std::iter::repeat(128u8).take((W * H / 2) as usize));
    let mut out = Vec::new();
    let dst = RectF {
        x: 2.0,
        y: 0.0,
        w: 4.0,
        h: H as f32,
    };
    blend_into(&mut out, &cur, &prev, W, H, full(), dst, 0.0);
    let band: Vec<u8> = out[2..6].to_vec();
    assert!(
        band.windows(2).all(|p| p[0] < p[1]),
        "still increasing left to right: {band:?}"
    );
    assert!(
        band[0] < 60 && band[3] > 170,
        "still spans the source's range: {band:?}"
    );
}
