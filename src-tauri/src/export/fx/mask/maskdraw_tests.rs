use super::*;

fn frame(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut v = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let c = f(x, y);
            let i = ((y * w + x) * 4) as usize;
            v[i] = c[2];
            v[i + 1] = c[1];
            v[i + 2] = c[0];
        }
    }
    v
}

fn px(out: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 4) as usize;
    [out[i + 2], out[i + 1], out[i]]
}

fn m(kind: u32) -> MaskDraw {
    MaskDraw {
        mn: [20.0, 20.0],
        mx: [80.0, 60.0],
        r: 0.0,
        feather_px: 1.0,
        amount_px: 6.0,
        dim: 0.6,
        kind,
        alpha: 1.0,
    }
}

#[test]
fn blur_leaves_the_outside_of_the_rect_untouched() {
    let (w, h) = (100u32, 80u32);
    let mut out = frame(w, h, |x, _| {
        if x % 2 == 0 {
            [0, 0, 0]
        } else {
            [255, 255, 255]
        }
    });
    let before = out.clone();
    draw_masks(&mut out, w, h, &[m(1)]);
    assert_eq!(
        px(&out, w, 5, 5),
        px(&before, w, 5, 5),
        "far outside is byte identical"
    );
    assert_eq!(
        px(&out, w, 95, 70),
        px(&before, w, 95, 70),
        "the other corner too"
    );
    let inside = px(&out, w, 50, 40);
    assert!(
        inside[0] > 40 && inside[0] < 215,
        "the stripes averaged to a mid grey: {inside:?}"
    );
}

#[test]
fn pixelate_produces_at_most_one_colour_per_cell() {
    let (w, h) = (100u32, 80u32);
    let mut out = frame(w, h, |x, y| [(x * 2) as u8, (y * 3) as u8, 0]);
    let mut mask = m(2);
    mask.amount_px = 10.0;
    mask.feather_px = 0.001;
    draw_masks(&mut out, w, h, &[mask]);
    for dx in 1..9u32 {
        assert_eq!(
            px(&out, w, 30 + dx, 30),
            px(&out, w, 31, 30),
            "every pixel of a ten px cell is the cell's colour"
        );
    }
    assert_ne!(
        px(&out, w, 31, 30),
        px(&out, w, 41, 30),
        "the next cell is a different colour"
    );
}

#[test]
fn highlight_darkens_the_outside_and_not_the_inside() {
    let (w, h) = (100u32, 80u32);
    let mut out = frame(w, h, |_, _| [200, 200, 200]);
    draw_masks(&mut out, w, h, &[m(3)]);
    assert_eq!(
        px(&out, w, 50, 40),
        [200, 200, 200],
        "inside keeps its brightness"
    );
    assert_eq!(
        px(&out, w, 5, 5),
        [80, 80, 80],
        "outside is multiplied by 1 - dim"
    );
}

#[test]
fn alpha_zero_is_a_byte_identical_no_op_for_every_kind() {
    let (w, h) = (64u32, 64u32);
    for kind in [1u32, 2, 3] {
        let mut out = frame(w, h, |x, y| {
            [(x * 5 + y * 7) as u8, (x * 3) as u8, (y * 11) as u8]
        });
        let before = out.clone();
        let mut mask = m(kind);
        mask.alpha = 0.0;
        draw_masks(&mut out, w, h, &[mask]);
        assert_eq!(out, before, "kind {kind} at alpha 0 touched the frame");
    }
}

#[test]
fn two_masks_read_the_same_source_frame() {
    let (w, h) = (100u32, 80u32);
    let mut one = frame(w, h, |x, _| {
        if x % 2 == 0 {
            [0, 0, 0]
        } else {
            [255, 255, 255]
        }
    });
    let mut two = one.clone();
    let a = m(1);
    let mut b = m(1);
    b.mn = [30.0, 30.0];
    b.mx = [70.0, 50.0];
    draw_masks(&mut one, w, h, &[a]);
    draw_masks(&mut two, w, h, &[a, b]);
    let solo = px(&one, w, 50, 40);
    let both = px(&two, w, 50, 40);
    assert!(
        (solo[0] as i32 - both[0] as i32).abs() <= 2,
        "the second blur re-blurred the first: {solo:?} vs {both:?}"
    );
}

#[test]
fn blur_sigma_is_the_three_box_approximation() {
    assert!((blur_sigma(1.0) - 2.0f32.sqrt()).abs() < 1e-5);
    assert!((blur_sigma(24.0) - (((49.0 * 49.0) - 1.0) / 4.0f32).sqrt()).abs() < 1e-3);
    assert_eq!(
        blur_sigma(0.0),
        blur_sigma(1.0),
        "the radius floors at one pixel"
    );
}
