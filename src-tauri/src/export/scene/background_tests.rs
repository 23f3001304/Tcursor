use super::*;
use crate::export::types::{Background, Rgb};
use crate::settings::background::{BackgroundKind, BackgroundSettings};

fn gradient(from: Rgb, to: Rgb, angle_deg: f32) -> Background {
    Background::Gradient {
        from,
        mid: None,
        to,
        angle_deg,
    }
}
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

#[test]
fn solid_fills_bgra() {
    let buf = render(
        &Background::Solid(Rgb {
            r: 10,
            g: 20,
            b: 30,
        }),
        2,
        2,
    );
    assert_eq!(buf.len(), 2 * 2 * 4);
    assert_eq!(&buf[0..4], &[30, 20, 10, 255]);
}

#[test]
fn gradient_differs_corner_to_corner() {
    let buf = render(&gradient(BLACK, WHITE, 0.0), 4, 1);
    assert!(buf[0] < buf[(3 * 4) as usize]);
}

#[test]
fn gradient_at_135deg_is_a_true_monotonic_ramp_not_mirror_folded() {
    let (w, h) = (100u32, 100u32);
    let buf = render(&gradient(BLACK, WHITE, 135.0), w, h);
    let px = |x: u32, y: u32| buf[((y * w + x) * 4) as usize];
    let (top_right, center, bottom_left) = (px(99, 0), px(50, 50), px(0, 99));
    assert!(
        top_right < center,
        "top-right ({top_right}) must be darker than center ({center})"
    );
    assert!(
        center < bottom_left,
        "center ({center}) must be darker than bottom-left ({bottom_left})"
    );
    assert_eq!(
        px(0, 0),
        px(99, 99),
        "the off-axis corners must be equal (both at t=0.5)"
    );
}

#[test]
fn a_middle_stop_lands_at_the_ramp_midpoint() {
    let (w, h) = (101u32, 1u32);
    let mid = Rgb {
        r: 200,
        g: 40,
        b: 10,
    };
    let buf = render(
        &Background::Gradient {
            from: BLACK,
            mid: Some(mid),
            to: WHITE,
            angle_deg: 0.0,
        },
        w,
        h,
    );
    let px = |x: u32, c: usize| buf[(x * 4) as usize + c] as i32;
    assert_eq!(
        (px(50, 2), px(50, 1), px(50, 0)),
        (200, 40, 10),
        "t=0.5 must be exactly the middle stop"
    );
    assert!(
        px(25, 2) > 90 && px(25, 2) < 110,
        "the first half ramps black -> mid, got {}",
        px(25, 2)
    );
    assert!(
        px(75, 1) > 140 && px(75, 1) < 155,
        "the second half ramps mid -> white, got {}",
        px(75, 1)
    );
}

#[test]
fn no_middle_stop_is_the_two_stop_ramp_unchanged() {
    let two = render(&gradient(BLACK, WHITE, 47.0), 32, 18);
    let explicit_none = render(
        &Background::Gradient {
            from: BLACK,
            mid: None,
            to: WHITE,
            angle_deg: 47.0,
        },
        32,
        18,
    );
    assert_eq!(
        two, explicit_none,
        "`mid: None` must be bit-identical to the pre-middle-stop ramp"
    );
}

#[test]
fn build_solid_matches_direct_render() {
    let s = BackgroundSettings {
        kind: BackgroundKind::Solid,
        solid: [10, 20, 30],
        ..Default::default()
    };
    let buf = build(&s, &[], 2, 2, Path::new("."));
    assert_eq!(
        buf,
        render(
            &Background::Solid(Rgb {
                r: 10,
                g: 20,
                b: 30
            }),
            2,
            2
        )
    );
}

#[test]
fn build_zero_blur_is_a_no_op() {
    let s = BackgroundSettings {
        kind: BackgroundKind::Gradient,
        blur: 0.0,
        ..Default::default()
    };
    let buf = build(&s, &[], 16, 16, Path::new("."));
    let direct = render(
        &gradient(
            rgb(s.gradient_from),
            rgb(s.gradient_to),
            s.gradient_angle_deg,
        ),
        16,
        16,
    );
    assert_eq!(buf, direct);
}

#[test]
fn build_passes_the_middle_stop_through_to_the_renderer() {
    let s = BackgroundSettings {
        kind: BackgroundKind::Gradient,
        gradient_from: [0, 0, 0],
        gradient_mid: Some([9, 9, 9]),
        gradient_to: [255, 255, 255],
        gradient_angle_deg: 0.0,
        ..Default::default()
    };
    let buf = build(&s, &[], 101, 1, Path::new("."));
    assert_eq!(
        buf[(50 * 4) as usize],
        9,
        "the settings' middle stop must reach `render`"
    );
}

#[test]
fn blur_softens_a_sharp_edge() {
    let (w, h) = (40u32, 40u32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 20..w {
            let i = ((y * w + x) * 4) as usize;
            buf[i] = 255;
            buf[i + 1] = 255;
            buf[i + 2] = 255;
            buf[i + 3] = 255;
        }
    }
    blur(&mut buf, w, h, 1.0);
    let i = ((20 * w + 19) * 4) as usize;
    assert!(buf[i] > 0, "edge should have softened into the dark side");
}

#[test]
fn blur_amount_zero_is_untouched() {
    let mut buf = vec![7u8, 8, 9, 255, 1, 2, 3, 255];
    let before = buf.clone();
    blur(&mut buf, 2, 1, 0.0);
    assert_eq!(buf, before);
}

fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "tcursor_bgs_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(p.join("background")).expect("create tempdir");
    p
}

#[test]
fn dim_multiplies_every_channel_and_leaves_alpha_alone() {
    let mut px = vec![100u8, 200, 255, 255];
    apply_dim(&mut px, 0.5);
    assert_eq!(px, vec![50, 100, 128, 255]);
    let mut none = vec![100u8, 200, 255, 255];
    apply_dim(&mut none, 0.0);
    assert_eq!(
        none,
        vec![100, 200, 255, 255],
        "dim 0 must not touch a single byte"
    );
    let mut full = vec![100u8, 200, 255, 255];
    apply_dim(&mut full, 1.0);
    assert_eq!(
        full,
        vec![0, 0, 0, 255],
        "alpha is never dimmed - this is an overlay, not a fade"
    );
}

#[test]
fn build_applies_the_clamped_dim_to_a_solid() {
    let s = BackgroundSettings {
        kind: BackgroundKind::Solid,
        solid: [200, 100, 50],
        dim: 9.0,
        ..Default::default()
    };
    let buf = build(&s, &[], 2, 2, Path::new("."));
    assert_eq!(&buf[..4], &[10, 20, 40, 255]);
}

#[test]
fn a_missing_asset_falls_back_to_the_base_wallpaper_instead_of_failing() {
    let dir = tempdir();
    let s = BackgroundSettings {
        kind: BackgroundKind::Image,
        asset: Some("background/gone.png".into()),
        solid: [1, 2, 3],
        ..Default::default()
    };
    let got = build(&s, &[], 4, 4, &dir);
    assert_eq!(
        got.len(),
        4 * 4 * 4,
        "a missing asset must still produce a full background"
    );
    assert!(
        video_source(&s, &dir).is_none(),
        "a missing file is never a decode source"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_source_is_only_the_video_kind_with_a_real_file() {
    let dir = tempdir();
    std::fs::write(dir.join("background/loop.mp4"), b"x").unwrap();
    let v = BackgroundSettings {
        kind: BackgroundKind::Video,
        asset: Some("background/loop.mp4".into()),
        ..Default::default()
    };
    assert_eq!(
        video_source(&v, &dir),
        Some(dir.join("background").join("loop.mp4"))
    );
    let i = BackgroundSettings {
        kind: BackgroundKind::Image,
        ..v.clone()
    };
    assert!(
        video_source(&i, &dir).is_none(),
        "an image never opens a decode stream"
    );
    let m = BackgroundSettings {
        kind: BackgroundKind::Mesh,
        ..v.clone()
    };
    assert!(
        video_source(&m, &dir).is_none(),
        "an asset kept while a wallpaper is showing stays idle"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
