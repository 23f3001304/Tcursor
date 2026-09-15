use super::*;

fn dib(px: &[[u8; 4]]) -> Vec<u8> {
    px.iter().flatten().copied().collect()
}
const BLACK: [u8; 4] = [0, 0, 0, 0];
const WHITE: [u8; 4] = [255, 255, 255, 0];

#[test]
fn mono_covers_the_whole_and_xor_table() {
    let mask = dib(&[WHITE, WHITE, BLACK, BLACK, BLACK, WHITE, BLACK, WHITE]);
    let out = mono_rgba(&mask, 2, 2, true).unwrap();
    assert_eq!(&out[0..4], &[0, 0, 0, 0], "AND=1,XOR=0 is transparent");
    assert_eq!(
        &out[4..8],
        &[255, 255, 255, 255],
        "AND=1,XOR=1 (invert) renders opaque white"
    );
    assert_eq!(&out[8..12], &[0, 0, 0, 255], "AND=0,XOR=0 is opaque black");
    assert_eq!(
        &out[12..16],
        &[255, 255, 255, 255],
        "AND=0,XOR=1 is opaque white"
    );
}

#[test]
fn mono_rejects_a_mask_that_is_not_twice_the_height() {
    assert!(mono_rgba(&dib(&[WHITE, WHITE]), 2, 1, true).is_none());
    assert!(mono_rgba(&[], 0, 0, true).is_none());
}

#[test]
fn color_keeps_real_alpha_and_swaps_bgr_to_rgb() {
    let src = dib(&[[10, 20, 30, 128]]);
    let out = color_rgba(&src, &[], 1, 1, true).unwrap();
    assert_eq!(out, vec![30, 20, 10, 128]);
}

#[test]
fn color_with_all_zero_alpha_falls_back_to_the_and_mask() {
    let src = dib(&[[0, 0, 255, 0], [0, 0, 255, 0]]);
    let mask = dib(&[WHITE, BLACK]);
    let out = color_rgba(&src, &mask, 2, 1, true).unwrap();
    assert_eq!(&out[0..4], &[255, 0, 0, 0], "mask bit 1 -> transparent");
    assert_eq!(&out[4..8], &[255, 0, 0, 255], "mask bit 0 -> opaque");
}

#[test]
fn color_with_all_zero_alpha_and_no_mask_stays_opaque() {
    let src = dib(&[[9, 9, 9, 0]]);
    assert_eq!(color_rgba(&src, &[], 1, 1, true).unwrap()[3], 255);
}

#[test]
fn bottom_up_rows_are_read_in_reverse() {
    let src = dib(&[[0, 0, 255, 255], [0, 255, 0, 255]]);
    let top = color_rgba(&src, &[], 1, 2, true).unwrap();
    assert_eq!(&top[0..4], &[255, 0, 0, 255]);
    assert_eq!(&top[4..8], &[0, 255, 0, 255]);
    let bottom = color_rgba(&src, &[], 1, 2, false).unwrap();
    assert_eq!(
        &bottom[0..4],
        &[0, 255, 0, 255],
        "bottom-up: buffer row 1 is the top scanline"
    );
    assert_eq!(&bottom[4..8], &[255, 0, 0, 255]);
}

#[test]
fn mono_bottom_up_reads_the_and_half_from_the_top_of_the_bitmap() {
    let bottom_up = dib(&[BLACK, WHITE]);
    let out = mono_rgba(&bottom_up, 1, 1, false).unwrap();
    assert_eq!(out, vec![0, 0, 0, 0], "AND=1,XOR=0 is transparent");
    let top_down = dib(&[WHITE, BLACK]);
    assert_eq!(mono_rgba(&top_down, 1, 1, true).unwrap(), out);
}

#[test]
fn a_short_colour_buffer_is_declined_rather_than_read_past() {
    assert!(color_rgba(&dib(&[[1, 2, 3, 4]]), &[], 4, 4, true).is_none());
}
