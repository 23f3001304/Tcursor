use super::*;

/// A `w x h` sprite, fully opaque white, hotspot at its centre (what every Crystal sprite has).
fn spr(w: u32, h: u32) -> CursorSprite {
    CursorSprite { bgra: vec![255; (w * h * 4) as usize], w, h, hot: (0.5, 0.5), canvas_h: h }
}

#[test]
fn a_sprite_box_puts_the_hotspot_on_the_cursor_point() {
    let b = sprite_box(&spr(20, 10), (100.0, 100.0), 10.0, 1.0);
    assert_eq!(b, [90.0, 95.0, 20.0, 10.0], "hotspot at the centre of a 20x10 box");
    // The bounce scales the box about the hotspot, not about the corner.
    let d = sprite_box(&spr(20, 10), (100.0, 100.0), 10.0, 0.5);
    assert_eq!([d[2], d[3]], [10.0, 5.0]);
    assert_eq!([d[0] + d[2] * 0.5, d[1] + d[3] * 0.5], [100.0, 100.0]);
}

#[test]
fn the_box_interpolates_from_the_previous_kind_to_the_new_one() {
    // A 40x40 disc becoming a 60x20 pill - the shapes Crystal actually morphs between.
    let (disc, pill) = (spr(40, 40), spr(60, 20));
    let at = |m: f32| morph_box(&disc, &pill, m, (100.0, 100.0), 40.0, 1.0);
    let (start, end) = (at(0.0), at(1.0));
    assert_eq!(start, sprite_box(&disc, (100.0, 100.0), 40.0, 1.0), "m=0 is the previous box");
    assert_eq!(end, sprite_box(&pill, (100.0, 100.0), 40.0, 1.0), "m=1 is the new one");
    let mid = at(0.5);
    for i in 0..4 {
        assert!((mid[i] - (start[i] + end[i]) * 0.5).abs() < 1e-4, "component {i} is the midpoint");
    }
    // And the hotspot stays on the cursor point the whole way through - a morph must not drift.
    for m in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let b = at(m);
        assert!((b[0] + b[2] * 0.5 - 100.0).abs() < 1e-4 && (b[1] + b[3] * 0.5 - 100.0).abs() < 1e-4);
    }
}

#[test]
fn a_settled_morph_takes_the_new_box_without_touching_the_old_sprite() {
    let (a, b) = (spr(4, 4), spr(60, 20));
    assert_eq!(morph_box(&a, &b, 1.0, (0.0, 0.0), 20.0, 1.0), sprite_box(&b, (0.0, 0.0), 20.0, 1.0));
    assert_eq!(morph_box(&a, &b, 1.5, (0.0, 0.0), 20.0, 1.0), sprite_box(&b, (0.0, 0.0), 20.0, 1.0));
}

#[test]
fn lerp_box_clamps_outside_zero_to_one() {
    let (a, b) = ([0.0; 4], [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(lerp_box(a, b, -1.0), a);
    assert_eq!(lerp_box(a, b, 2.0), b);
    assert_eq!(lerp_box(a, b, 0.5), [5.0, 10.0, 15.0, 20.0]);
}
