use super::*;
use crate::edit::model::{EffectKind, EffectRegion};
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, RectF};

const SRC: RectF = RectF {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1080.0,
};

fn panel(x: f32, y: f32, w: f32, h: f32, alpha: f32) -> Panel {
    Panel {
        rect: RectF { x, y, w, h },
        radius: 0.0,
        alpha,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    }
}

fn scene_screen() -> Scene {
    Scene {
        screen: panel(0.0, 0.0, 1920.0, 1080.0, 1.0),
        camera: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        src: SRC,
    }
}

fn scene_presenter() -> Scene {
    Scene {
        screen: panel(96.0, 108.0, 1344.0, 756.0, 1.0),
        camera: panel(1480.0, 620.0, 360.0, 360.0, 1.0),
        src: SRC,
    }
}

fn mask(kind: EffectKind, rect: [f32; 4]) -> EffectRegion {
    EffectRegion {
        id: "m0".into(),
        kind,
        start_ms: 0,
        end_ms: 10_000,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
        rect: Some(rect),
        strength: None,
        roundness: None,
    }
}

fn cam(scale: f32, cx: f32, cy: f32) -> Camera {
    Camera { cx, cy, scale }
}

#[test]
fn masks_at_fades_each_region_independently() {
    let mut a = mask(EffectKind::Blur, [0.1, 0.1, 0.2, 0.2]);
    a.start_ms = 0;
    a.end_ms = 2000;
    a.fade_in_ms = 1000;
    let mut b = mask(EffectKind::Pixelate, [0.5, 0.5, 0.2, 0.2]);
    b.id = "m1".into();
    b.start_ms = 0;
    b.end_ms = 2000;
    b.fade_in_ms = 200;
    b.layer = 1;
    let got = masks_at(
        &[a, b],
        &scene_screen(),
        cam(1.0, 960.0, 540.0),
        SRC,
        1920,
        1080,
        400,
        0.6,
    );
    assert_eq!(got.len(), 2, "masks compose, they do not elect a winner");
    assert!(
        (got[0].alpha - 0.4).abs() < 1e-3,
        "the slow fade is 40 percent in: {}",
        got[0].alpha
    );
    assert_eq!(got[1].alpha, 1.0, "the fast fade is already full");
    assert_eq!((got[0].kind, got[1].kind), (1, 2));
}

#[test]
fn a_mask_on_a_hidden_screen_panel_draws_nothing() {
    let mut s = scene_screen();
    s.screen.alpha = 0.2;
    let got = masks_at(
        &[mask(EffectKind::Blur, [0.1, 0.1, 0.2, 0.2])],
        &s,
        cam(1.0, 960.0, 540.0),
        SRC,
        1920,
        1080,
        5000,
        0.6,
    );
    assert!(
        got.is_empty(),
        "camera_only means the screen is not drawn, so neither is a mask on it"
    );
}

#[test]
fn a_spotlight_region_and_a_rectless_mask_are_never_returned() {
    let spot = mask(EffectKind::Spotlight, [0.1, 0.1, 0.2, 0.2]);
    let mut bare = mask(EffectKind::Blur, [0.1, 0.1, 0.2, 0.2]);
    bare.rect = None;
    let got = masks_at(
        &[spot, bare],
        &scene_screen(),
        cam(1.0, 960.0, 540.0),
        SRC,
        1920,
        1080,
        5000,
        0.6,
    );
    assert!(
        got.is_empty(),
        "a spotlight is not a mask, and a mask with no rect covers nothing"
    );
}

#[test]
fn a_rect_projected_off_the_panel_is_clipped_away() {
    let mut s = scene_screen();
    s.src = RectF {
        x: 960.0,
        y: 0.0,
        w: 960.0,
        h: 1080.0,
    };
    let got = masks_at(
        &[mask(EffectKind::Blur, [0.02, 0.02, 0.05, 0.05])],
        &s,
        cam(1.0, 960.0, 540.0),
        SRC,
        1920,
        1080,
        5000,
        0.6,
    );
    assert!(
        got.is_empty(),
        "a rect on display A while display B is live lands outside the panel"
    );
}

#[test]
fn defaults_come_from_the_kind_and_the_global_dim() {
    let got = masks_at(
        &[
            mask(EffectKind::Blur, [0.1, 0.1, 0.2, 0.2]),
            mask(EffectKind::Pixelate, [0.1, 0.1, 0.2, 0.2]),
            mask(EffectKind::Highlight, [0.1, 0.1, 0.2, 0.2]),
        ],
        &scene_screen(),
        cam(1.0, 960.0, 540.0),
        SRC,
        1920,
        1080,
        5000,
        0.62,
    );
    assert_eq!(got.len(), 3);
    assert!(
        (got[0].amount_px - 1080.0 * 0.020).abs() < 1e-3,
        "blur takes DEFAULT_BLUR"
    );
    assert!(
        (got[1].amount_px - 1080.0 * 0.018).abs() < 1e-3,
        "pixelate takes DEFAULT_PIXEL"
    );
    assert!(
        (got[2].dim - 0.62).abs() < 1e-6,
        "highlight takes the global spotlight dim"
    );
    assert!(
        (got[0].feather_px - 1080.0 * 0.010).abs() < 1e-3,
        "DEFAULT_MASK_FEATHER"
    );
    assert!(
        (got[0].r - 216.0 * 0.06).abs() < 1e-3,
        "roundness is a fraction of the SHORT side"
    );
}

#[path = "fx_masks_table_tests.rs"]
mod table_tests;
