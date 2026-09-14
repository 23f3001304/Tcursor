use super::*;
use std::sync::Arc;
use crate::export::fx::fx_lens::LensMask;

fn frame(w: u32, h: u32, v: u8) -> Vec<u8> { vec![v; (w * h * 4) as usize] }
fn at(out: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 4) as usize;
    [out[i], out[i + 1], out[i + 2]]
}

fn disc(cx: f32, cy: f32, r: f32) -> BackLens {
    BackLens { mn: [cx - r, cy - r], mx: [cx + r, cy + r], r, squash: 1.0, ink: -1.0,
        ink_at: [cx, cy], ring: false }
}

#[test]
fn the_back_tints_inside_and_shadows_below() {
    let (w, h) = (80u32, 80u32);
    let mut out = frame(w, h, 100);
    draw_lens(&mut out, w, h, &Lenses { glass: None, back: Some(disc(40.0, 40.0, 12.0)) }, [255, 0, 0]);
    assert!(at(&out, w, 40, 40)[0] > 100, "the disc lightens what is under it");
    // Two px below the disc's bottom edge the shadow darkens the frame instead.
    assert!(at(&out, w, 40, 53)[0] < 100, "a drop shadow under the rim");
    assert_eq!(at(&out, w, 4, 4), [100, 100, 100], "nothing outside the shape is touched");
}

#[test]
fn a_live_ink_drop_paints_the_accent_inside_the_back() {
    let (w, h) = (80u32, 80u32);
    let mut out = frame(w, h, 100);
    let b = BackLens { ink: 0.35, ..disc(40.0, 40.0, 14.0) };
    draw_lens(&mut out, w, h, &Lenses { glass: None, back: Some(b) }, [0, 0, 255]);
    let p = at(&out, w, 40, 40);
    assert!(p[0] > p[2], "blue accent (B at BGRA idx 0) over the neutral tint, got {p:?}");
}

#[test]
fn the_glass_lens_only_touches_pixels_the_sprite_alpha_covers() {
    let (w, h) = (60u32, 60u32);
    // A mask that is opaque on its left half and empty on its right.
    let mask = Arc::new(LensMask { key: 1, w: 2, h: 1, a: vec![255, 0] });
    let g = CursorLens { cbox: [30.0, 30.0, 20.0, 20.0], angle: 0.0, squash: 1.0, ink: -1.0,
        ink_at: [30.0, 30.0], mask };
    let mut out = frame(w, h, 100);
    draw_lens(&mut out, w, h, &Lenses { glass: Some(g), back: None }, [255, 0, 0]);
    assert!(at(&out, w, 25, 30)[0] > 100, "the covered half is tinted");
    assert_eq!(at(&out, w, 35, 30), [100, 100, 100], "the transparent half is left alone");
}

#[test]
fn an_empty_lens_pair_leaves_the_frame_untouched() {
    let (w, h) = (32u32, 32u32);
    let mut out = frame(w, h, 7);
    draw_lens(&mut out, w, h, &Lenses { glass: None, back: None }, [255, 0, 0]);
    assert!(out.iter().all(|&b| b == 7));
}

/// The magnifier: a thin vertical stripe under the disc's centre comes out `ZOOM` wider - the
/// pixel one stripe-width out, which was background before, now shows the stripe.
#[test]
fn the_back_magnifies_what_is_under_it_about_its_centre() {
    let (w, h) = (80u32, 80u32);
    let mut out = frame(w, h, 100);
    for y in 0..h { for x in 38..42u32 { let i = ((y * w + x) * 4) as usize; out[i] = 250; out[i + 1] = 250; out[i + 2] = 250; } }
    let before = at(&out, w, 42, 40);
    assert_eq!(before, [100, 100, 100], "x=42 is background before the lens");
    draw_lens(&mut out, w, h, &Lenses { glass: None, back: Some(disc(40.0, 40.0, 20.0)) }, [0, 0, 0]);
    assert!(at(&out, w, 42, 40)[1] > 200, "inside the disc the stripe is ~1.35x wider: {:?}", at(&out, w, 42, 40));
    assert!(at(&out, w, 50, 40)[1] < 130, "but not everywhere: {:?}", at(&out, w, 50, 40));
    assert_eq!(at(&out, w, 4, 40), [100, 100, 100], "nothing outside the shape is touched");
}
