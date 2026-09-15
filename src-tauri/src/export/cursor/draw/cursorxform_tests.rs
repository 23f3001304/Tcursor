use super::*;

const OW: u32 = 12;
const CLIP: (i32, i32, i32, i32) = (0, 0, OW as i32, OW as i32);

const ANCHOR: (f32, f32) = (5.5, 5.5);

fn ids_3x3() -> CursorSprite {
    let mut bgra = Vec::new();
    for y in 0..3u8 {
        for x in 0..3u8 {
            bgra.extend_from_slice(&[0, 0, 10 * (y * 3 + x) + 10, 255]);
        }
    }
    CursorSprite {
        bgra,
        w: 3,
        h: 3,
        hot: (0.5, 0.5),
        canvas_h: 3,
    }
}

fn frame() -> Vec<u8> {
    vec![0u8; (OW * OW * 4) as usize]
}

fn id(out: &[u8], x: u32, y: u32) -> u8 {
    out[((y * OW + x) * 4 + 2) as usize]
}

#[test]
fn a_quarter_turn_clockwise_maps_every_texel_exactly() {
    let mut out = frame();
    blit_transformed(
        &mut out,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        90.0,
        1.0,
        CLIP,
        1.0,
    );
    for i in 0..3u32 {
        for j in 0..3u32 {
            let want = 10 * ((2 - i) * 3 + j) as u8 + 10;
            assert_eq!(
                id(&out, 4 + i, 4 + j),
                want,
                "output ({}, {})",
                4 + i,
                4 + j
            );
        }
    }
    assert_eq!(id(&out, 3, 4), 0);
    assert_eq!(id(&out, 7, 5), 0);
}

#[test]
fn a_full_turn_is_the_identity_and_a_half_turn_reverses_both_axes() {
    let mut full = frame();
    blit_transformed(
        &mut full,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        360.0,
        1.0,
        CLIP,
        1.0,
    );
    let mut none = frame();
    blit_transformed(
        &mut none,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        0.0,
        1.0,
        CLIP,
        1.0,
    );
    assert_eq!(
        full, none,
        "360 degrees lands back on the untransformed placement"
    );

    let mut half = frame();
    blit_transformed(
        &mut half,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        180.0,
        1.0,
        CLIP,
        1.0,
    );
    for i in 0..3u32 {
        for j in 0..3u32 {
            assert_eq!(
                id(&half, 4 + i, 4 + j),
                id(&none, 6 - i, 6 - j),
                "180 flips both axes"
            );
        }
    }
}

#[test]
fn the_hotspot_stays_on_the_anchor_through_a_pulse() {
    for extra in [1.0f32, 1.06, 2.0] {
        let mut out = frame();
        blit_transformed(
            &mut out,
            OW,
            OW,
            &ids_3x3(),
            ANCHOR,
            1.0,
            0.0,
            extra,
            CLIP,
            1.0,
        );
        assert_eq!(id(&out, 5, 5), 50, "anchor pixel at extra={extra}");
    }
}

#[test]
fn scaling_up_widens_the_footprint_around_the_anchor() {
    let mut small = frame();
    blit_transformed(
        &mut small,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        0.0,
        1.0,
        CLIP,
        1.0,
    );
    let mut big = frame();
    blit_transformed(
        &mut big,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        0.0,
        2.0,
        CLIP,
        1.0,
    );
    let painted = |o: &[u8]| (0..OW * OW).filter(|i| o[(i * 4 + 3) as usize] > 0).count();
    assert_eq!(
        painted(&small),
        9,
        "a 3x3 sprite at scale 1 covers exactly 9 pixels"
    );
    assert!(
        painted(&big) > 30,
        "doubled it covers about 36, got {}",
        painted(&big)
    );
}

#[test]
fn the_clip_box_confines_the_transformed_blit() {
    let mut out = frame();
    blit_transformed(
        &mut out,
        OW,
        OW,
        &ids_3x3(),
        ANCHOR,
        1.0,
        90.0,
        1.0,
        (0, 0, 5, OW as i32),
        1.0,
    );
    assert_ne!(id(&out, 4, 4), 0, "inside the clip");
    assert_eq!(id(&out, 5, 4), 0, "past the clip edge");
    assert_eq!(id(&out, 6, 4), 0);
}

#[test]
fn degenerate_inputs_are_a_safe_no_op() {
    let blank = frame();
    for (scale, extra, angle) in [
        (0.0f32, 1.0f32, 45.0f32),
        (1.0, 0.0, 45.0),
        (-1.0, 1.0, 0.0),
    ] {
        let mut out = frame();
        blit_transformed(
            &mut out,
            OW,
            OW,
            &ids_3x3(),
            ANCHOR,
            scale,
            angle,
            extra,
            CLIP,
            1.0,
        );
        assert_eq!(out, blank, "scale={scale} extra={extra}");
    }
    let mut out = frame();
    blit_transformed(
        &mut out,
        OW,
        OW,
        &ids_3x3(),
        (900.0, 900.0),
        1.0,
        33.0,
        1.0,
        CLIP,
        1.0,
    );
    assert_eq!(out, blank);
}

#[test]
fn a_rotated_edge_blends_without_a_halo_from_transparent_padding() {
    let mut bgra = vec![0u8; 3 * 3 * 4];
    let c = (1 * 3 + 1) * 4;
    bgra[c..c + 4].copy_from_slice(&[255, 255, 255, 255]);
    let spr = CursorSprite {
        bgra,
        w: 3,
        h: 3,
        hot: (0.5, 0.5),
        canvas_h: 3,
    };
    let mut out = frame();
    blit_transformed(
        &mut out,
        OW,
        OW,
        &spr,
        (5.9, 5.9),
        1.0,
        45.0,
        1.0,
        CLIP,
        1.0,
    );
    let mut touched = 0;
    for i in 0..OW * OW {
        let (o, a) = ((i * 4) as usize, out[(i * 4 + 3) as usize]);
        if a == 0 {
            continue;
        }
        touched += 1;
        for c in 0..3 {
            assert!(
                out[o + c] + 1 >= a,
                "pixel {i} lost {} of its colour to the transparent padding: {:?}",
                a - out[o + c],
                &out[o..o + 4]
            );
        }
    }
    assert!(
        touched > 1,
        "the rotated texel should straddle several pixels, got {touched}"
    );
}
