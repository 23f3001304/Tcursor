// Multi-resolution / non-16:9-aspect parity tests (T34 L2, folded in from L1's review). L1's own
// parity suite (`arrangement_tests.rs`) only ever resolved at a fixed 1920x1080 output. This
// proves the load-bearing claim - "converting a preset to an arrangement doesn't visibly move the
// picture" - also holds at a second output RESOLUTION and at non-16:9 output ASPECTS, both real
// reachable configs (`export::types::Layout::resolve` maps `Aspect::Vertical9x16`/`Square1x1` to
// concrete `out_w`/`out_h` - see that module's own tests), so L3 can rely on the claim beyond the
// one size L1 checked.
use super::*;
use crate::actions::model::LayoutId;
use crate::export::scene::resolve;
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

const PRESETS: [LayoutId; 5] = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter,
    LayoutId::ScreenOnly, LayoutId::CameraOnly];
// A realistic captured screen, held fixed across every case below - the OUTPUT aspect/resolution
// under test is independent of the source capture (a user can export any output aspect from any
// capture; the export fits/backgrounds around it).
const SW: u32 = 1920;
const SH: u32 = 1080;

fn resolved(app: &AppearanceSettings, id: LayoutId, ow: u32, oh: u32)
    -> (Scene, crate::export::types::Layout, crate::export::types::OverlayLayout) {
    let ma = app.for_id(id);
    let (l, ov) = (layout_for(ma, ow, oh), overlay_for(ma, ow, oh, true));
    (resolve(id, &l, &ov, SW, SH), l, ov)
}

/// Largest absolute per-field difference between two scenes, in output pixels - same yardstick
/// `arrangement_tests.rs` uses (alpha counts as 1.0 == one pixel, so a visibility mismatch can
/// never hide inside the tolerance).
fn max_dev(a: &Scene, b: &Scene) -> f32 {
    let per = |p: &Panel, q: &Panel| [
        (p.rect.x - q.rect.x).abs(), (p.rect.y - q.rect.y).abs(),
        (p.rect.w - q.rect.w).abs(), (p.rect.h - q.rect.h).abs(),
        (p.radius - q.radius).abs(), (p.alpha - q.alpha).abs(), (p.ring_px - q.ring_px).abs(),
    ].into_iter().fold(0.0f32, f32::max);
    per(&a.screen, &b.screen).max(per(&a.camera, &b.camera))
}

/// Worst-case round-trip deviation (px) over all 5 presets at one `(ow, oh)` output frame, default
/// appearance. Asserts every preset stays under the SAME 0.5px tolerance L1 used - not loosened.
fn parity_at(ow: u32, oh: u32, tag: &str) -> f32 {
    let app = AppearanceSettings::default();
    let mut worst = 0.0f32;
    for id in PRESETS {
        let (preset, l, ov) = resolved(&app, id, ow, oh);
        let arr = arrangement_of_preset(&preset, ow as f32, oh as f32);
        let posed = resolve_arrangement(&arr, preset, &l, &ov, SW, SH);
        let d = max_dev(&preset, &posed);
        assert!(d < 0.5, "{tag} {id:?} @ {ow}x{oh}: {d}px deviation\n preset {preset:?}\n posed  {posed:?}");
        worst = worst.max(d);
    }
    worst
}

/// A second output RESOLUTION (still 16:9, but smaller than L1's 1920x1080 - the exact size
/// `Resolution::P720` downscales 1080p to, per `types.rs`'s own tests).
#[test]
fn parity_holds_at_a_second_output_resolution_1280x720() {
    let worst = parity_at(1280, 720, "1280x720");
    println!("PARITY 1280x720 (16:9): max deviation {worst}px");
}

/// A non-16:9 output ASPECT: vertical 9:16 (`Aspect::Vertical9x16` -> 1080x1920).
#[test]
fn parity_holds_at_a_vertical_9x16_output_aspect() {
    let worst = parity_at(1080, 1920, "1080x1920 vertical");
    println!("PARITY 1080x1920 (vertical 9:16): max deviation {worst}px");
}

/// A non-16:9 output ASPECT: square 1:1 (`Aspect::Square1x1` -> 1080x1080).
#[test]
fn parity_holds_at_a_square_1x1_output_aspect() {
    let worst = parity_at(1080, 1080, "1080x1080 square");
    println!("PARITY 1080x1080 (square 1:1): max deviation {worst}px");
}
