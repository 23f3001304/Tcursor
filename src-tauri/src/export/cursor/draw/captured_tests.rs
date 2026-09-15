use super::*;
use crate::events::track::cursorlayer::CursorLayerBuilder;
use crate::events::track::cursorpixels::CapturedCursor;
use crate::export::types::RectF;

const OUT: u32 = 40;

fn tmp(tag: &str) -> ProjectPaths {
    let dir = std::env::temp_dir().join(format!("tcursor-captured-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ProjectPaths { folder: dir }
}

fn full_panel() -> Panel {
    Panel {
        rect: RectF {
            x: 0.0,
            y: 0.0,
            w: OUT as f32,
            h: OUT as f32,
        },
        radius: 0.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    }
}

fn red_2x2() -> CapturedCursor {
    red(2, 1, 1)
}

fn red(n: u32, hx: u32, hy: u32) -> CapturedCursor {
    let rgba = std::iter::repeat([255u8, 0, 0, 255])
        .take((n * n) as usize)
        .flatten()
        .collect();
    CapturedCursor {
        w: n,
        h: n,
        hx,
        hy,
        rgba,
    }
}

fn loaded(tag: &str, c: CapturedCursor) -> (ProjectPaths, CapturedCursors) {
    let paths = tmp(tag);
    let mut b = CursorLayerBuilder::default();
    let id = b.add(c);
    b.mark(0, id);
    b.save(&paths).unwrap();
    let cc = CapturedCursors::load(&paths).unwrap();
    (paths, cc)
}

fn px(out: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * OUT + x) * 4) as usize;
    [out[i], out[i + 1], out[i + 2], out[i + 3]]
}

#[test]
fn the_sprite_lands_with_its_hotspot_on_the_raw_recorded_point() {
    let paths = tmp("hotspot");
    let mut b = CursorLayerBuilder::default();
    let id = b.add(red_2x2());
    b.mark(0, id);
    b.save(&paths).unwrap();

    let cc = CapturedCursors::load(&paths).unwrap();
    let mut out = vec![0u8; (OUT * OUT * 4) as usize];
    let cam = Camera {
        cx: OUT as f32 / 2.0,
        cy: OUT as f32 / 2.0,
        scale: 1.0,
    };
    cc.draw(
        &mut out,
        OUT,
        OUT,
        FramePoint { x: 10, y: 10 },
        cam,
        &full_panel(),
        OUT as f32,
        OUT,
        0,
    );

    assert_eq!(
        px(&out, 9, 9),
        [0, 0, 255, 255],
        "sprite top-left at (point - hotspot)"
    );
    assert_eq!(
        px(&out, 10, 10),
        [0, 0, 255, 255],
        "hotspot pixel on the recorded point"
    );
    assert_eq!(
        px(&out, 8, 9),
        [0, 0, 0, 0],
        "nothing is drawn left of the sprite"
    );
    assert_eq!(px(&out, 11, 10), [0, 0, 0, 0], "nor past its right edge");
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn nothing_is_drawn_before_the_first_sample_or_with_the_panel_faded_out() {
    let paths = tmp("gates");
    let mut b = CursorLayerBuilder::default();
    let id = b.add(red_2x2());
    b.mark(500, id);
    b.save(&paths).unwrap();

    let cc = CapturedCursors::load(&paths).unwrap();
    assert!(cc.sprite_at(499).is_none());
    assert!(cc.sprite_at(500).is_some());

    let cam = Camera {
        cx: OUT as f32 / 2.0,
        cy: OUT as f32 / 2.0,
        scale: 1.0,
    };
    let mut early = vec![0u8; (OUT * OUT * 4) as usize];
    cc.draw(
        &mut early,
        OUT,
        OUT,
        FramePoint { x: 10, y: 10 },
        cam,
        &full_panel(),
        OUT as f32,
        OUT,
        0,
    );
    assert!(early.iter().all(|&b| b == 0), "no sample yet -> no cursor");

    let mut faded_panel = full_panel();
    faded_panel.alpha = 0.4;
    let mut faded = vec![0u8; (OUT * OUT * 4) as usize];
    cc.draw(
        &mut faded,
        OUT,
        OUT,
        FramePoint { x: 10, y: 10 },
        cam,
        &faded_panel,
        OUT as f32,
        OUT,
        500,
    );
    assert!(
        faded.iter().all(|&b| b == 0),
        "no screen panel -> no cursor"
    );
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn a_recording_with_no_layer_falls_back_to_the_synthetic_arrow() {
    let paths = tmp("nolayer");
    assert!(CapturedCursors::load(&paths).is_none());
    assert!(!draws_captured(CursorStyle::System, false));
    assert!(crate::export::cursor::pack::cursorset::draws_synthetic(
        &crate::settings::cursor::CursorSettings::default(),
        false
    ));
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn a_source_twice_the_output_draws_the_cursor_at_half_size() {
    let cam = Camera {
        cx: OUT as f32 / 2.0,
        cy: OUT as f32 / 2.0,
        scale: 1.0,
    };
    let point = FramePoint { x: 10, y: 10 };
    let frame = || vec![0u8; (OUT * OUT * 4) as usize];

    let (p1, one_to_one) = loaded("scale1x", red(4, 2, 2));
    let mut full = frame();
    one_to_one.draw(
        &mut full,
        OUT,
        OUT,
        point,
        cam,
        &full_panel(),
        OUT as f32,
        OUT,
        0,
    );
    assert_eq!(
        px(&full, 8, 8),
        [0, 0, 255, 255],
        "1:1 source covers its full 4x4"
    );
    assert_eq!(px(&full, 11, 11), [0, 0, 255, 255]);

    let (p2, two_to_one) = loaded("scale2x", red(4, 2, 2));
    let mut half = frame();
    two_to_one.draw(
        &mut half,
        OUT,
        OUT,
        point,
        cam,
        &full_panel(),
        OUT as f32,
        OUT * 2,
        0,
    );
    assert_eq!(
        px(&half, 9, 9),
        [0, 0, 255, 255],
        "half-scale top-left at (point - hotspot/2)"
    );
    assert_eq!(
        px(&half, 10, 10),
        [0, 0, 255, 255],
        "hotspot still lands on the recorded point"
    );
    assert_eq!(
        px(&half, 8, 8),
        [0, 0, 0, 0],
        "half-scale must not reach the 1:1 corner"
    );
    assert_eq!(px(&half, 11, 11), [0, 0, 0, 0], "nor its opposite corner");
    for p in [p1, p2] {
        let _ = std::fs::remove_dir_all(&p.folder);
    }
}

#[test]
fn content_scale_is_the_source_to_panel_ratio_with_the_panel_shrink_applied_once() {
    assert_eq!(
        content_scale(1.0, 1920.0, 1920),
        1.0,
        "same size in and out -> untouched"
    );
    assert_eq!(
        content_scale(1.0, 1920.0, 3840),
        0.5,
        "a 4K source in a 1080p frame -> half"
    );
    assert_eq!(
        content_scale(1.0, 1920.0, 960),
        2.0,
        "an upscaled source grows with its content"
    );
    assert_eq!(content_scale(0.5, 1920.0, 1920), 0.5);
    assert_eq!(content_scale(0.5, 1920.0, 3840), 0.25);
    assert!(
        content_scale(1.0, 1920.0, 0).is_finite(),
        "a degenerate source width cannot divide by zero"
    );
}

#[test]
fn only_system_with_a_layer_draws_the_captured_cursor() {
    let cases = [
        (CursorStyle::System, true, true),
        (CursorStyle::System, false, false),
        (CursorStyle::Enhanced, true, false),
        (CursorStyle::Enhanced, false, false),
        (CursorStyle::Hidden, true, false),
        (CursorStyle::Hidden, false, false),
    ];
    for (style, layer, want) in cases {
        assert_eq!(
            draws_captured(style, layer),
            want,
            "{style:?} layer={layer}"
        );
    }
}
