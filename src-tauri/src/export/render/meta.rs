// Small data-only types returned by FrameRenderer, split out of mod.rs (which was at the 200-line
// budget) purely for size - no behavior lives here. Re-exported from `render::mod` so callers keep
// using the `render::` path.
use crate::export::pipeline::timeline::Timeline;
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint};

/// What the export loop needs to set up its decoders and drive the frame loop.
pub struct RenderMeta {
    pub tl: Timeline, pub video_start: u64, pub video_end: u64, pub out_w: u32, pub out_h: u32,
    pub sw: u32, pub sh: u32, pub screen_bytes: usize, pub audio_offset_ms: i32,
    pub screen_crop: Option<(u32, u32)>, // an odd-sized capture's exact even crop (`even_screen`), else None
    pub webcam_w: u32, pub webcam_h: u32, // webcam decode box: the SOURCE's aspect (see `webcam_box`)
    pub trim: crate::edit::model::Trim, // unresolved (`Trim::resolve`); avoids a 2nd doc load in `exporter::export`
    pub mic_volume: f32, pub sys_volume: f32, // 0..1.5 gain per track at mux (`audio_mux::mux`)
}

/// The screen decode's dims and, when the capture is odd-sized, the exact crop that evens it.
/// nv12 has no odd sizes: ffmpeg pads the chroma plane and a frame no longer measures `w*h*3/2`
/// bytes, so an odd 1697x955 window capture was read off the pipe a fraction of a row late on
/// every frame and the whole export slid and sheared (2026-09-14). Dropping one column or row of
/// a screen capture is invisible; resampling it would not be. `(2, 2)` floors a degenerate probe.
pub fn even_screen(w: u32, h: u32) -> (u32, u32, Option<(u32, u32)>) {
    let (ew, eh) = ((w & !1).max(2), (h & !1).max(2));
    (ew, eh, ((ew, eh) != (w, h)).then_some((ew, eh)))
}

/// Camera + scene for one output frame, carrying BOTH clocks: `ev_t` indexes the raw event streams
/// (cursor/mouse/actions), `out_t` is the timeline every `EditDoc` region list lives on.
///
/// `mix` and `hold` are the display-switch cross-dissolve's two halves, and are `None` on every
/// frame of a take that never switched. `mix` says this frame is inside a switch and wants the
/// held pre-switch screen frame blended in; `hold` (the span index) says THIS frame's screen
/// buffer is the one to latch, because the next output frame is already past the switch.
pub struct FramePose { pub ev_t: u32, pub out_t: u32, pub scene: Scene, pub cur: FramePoint, pub cam: Camera,
    pub mix: Option<crate::export::render::spans::SpanMix>, pub hold: Option<usize> }

/// The ONE webcam decode box for a whole export, at the SOURCE's own aspect (`src`, from
/// `probe_dims`) - NOT any panel's. The layout track can put a Wide bubble and a square big-cam
/// in the same export, so no single panel-shaped box is right for all of them; the compositors
/// cover-crop this box to each panel's aspect per frame instead (`gpu::compositor::cover_rect`
/// / `shader.wgsl`), which is exactly what the source-aspect box makes possible: cropping the
/// full source to 16:9 or to 1:1 both stay inside it, while a square box has already thrown the
/// sides away and could only ever give a Wide panel a zoomed-in band.
///
/// Sized to cover the largest panel on BOTH axes (`panels` = every mode's resolved overlay:
/// square modes draw `size_px` wide, bubble modes `width_px`), then clamped to `cap` and to the
/// source's own height - decoding above the source resolution just makes ffmpeg upscale with
/// `fast_bilinear` where the compositor would do it better. Dims are even for the decoder.
/// `src` `None` (no webcam, or an unreadable one) falls back to a square box.
pub fn webcam_box(panels: &[crate::export::types::OverlayLayout], src: Option<(u32, u32)>, cap: u32) -> (u32, u32) {
    let need_w = panels.iter().map(|o| o.width_px.max(o.size_px)).max().unwrap_or(420).max(2);
    let need_h = panels.iter().map(|o| o.size_px).max().unwrap_or(420).max(2);
    let (sw, sh) = src.filter(|(w, h)| *w >= 2 && *h >= 2).unwrap_or((cap, cap));
    let aspect = sw as f32 / sh as f32;
    let h = need_h.max((need_w as f32 / aspect).ceil() as u32).min(cap).min(sh);
    let even = |v: u32| (v & !1).max(2);
    (even((h as f32 * aspect).round() as u32), even(h))
}

#[cfg(test)]
mod tests {
    use super::webcam_box;
    use crate::settings::appearance::{overlay_for, AppearanceSettings, CamAspect};
    use crate::export::types::OverlayLayout;

    /// The five resolved overlays for a 16:9 export `oh` tall, bubble modes set to `aspect`.
    fn panels_at(aspect: CamAspect, oh: u32) -> Vec<OverlayLayout> {
        let mut a = AppearanceSettings::default();
        a.screen.cam_aspect = aspect;
        a.screen_only.cam_aspect = aspect;
        [&a.screen, &a.screen_only, &a.camera, &a.camera_only, &a.presenter]
            .iter().map(|m| overlay_for(m, oh * 16 / 9, oh, true)).collect()
    }
    fn panels(aspect: CamAspect) -> Vec<OverlayLayout> { panels_at(aspect, 1080) }

    /// The box carries the SOURCE's aspect, not any panel's - a mixed-layout export (Wide bubble
    /// + square big-cam) has no single panel aspect, and each panel crops this box at composite
    /// time. A 1280x720 webcam therefore decodes as the whole 1280x720 frame.
    #[test]
    fn the_box_takes_the_sources_aspect_not_a_panels() {
        for aspect in [CamAspect::Square, CamAspect::Wide] {
            let (w, h) = webcam_box(&panels(aspect), Some((1280, 720)), 1440);
            assert_eq!((w, h), (1280, 720), "{aspect:?}");
        }
        // A 4:3 webcam stays 4:3 (nothing forces 16:9 or a square on it).
        let (w, h) = webcam_box(&panels(CamAspect::Square), Some((640, 480)), 1440);
        assert_eq!((w, h), (640, 480));
    }

    /// It never decodes above the source resolution (the compositor upscales better than
    /// ffmpeg's `fast_bilinear`), and never above `cap`.
    #[test]
    fn the_box_is_clamped_by_the_source_and_the_cap() {
        // A 4K export wants a 1920px-tall big-cam panel; the cap holds the decode to 1440.
        assert_eq!(webcam_box(&panels_at(CamAspect::Square, 2160), Some((3840, 2160)), 1440).1, 1440);
        assert_eq!(webcam_box(&panels(CamAspect::Square), Some((320, 240)), 1440), (320, 240));
    }

    /// Big enough for the biggest panel on BOTH axes: a tall source must still be wide enough
    /// for a 16:9 bubble, so the height grows past `size_px` to buy that width.
    #[test]
    fn the_box_covers_the_widest_panel_on_a_tall_source() {
        let p = panels(CamAspect::Wide);
        let need_w = p.iter().map(|o| o.width_px.max(o.size_px)).max().unwrap();
        let (w, h) = webcam_box(&p, Some((1080, 1920)), 4096); // 9:16 portrait webcam
        assert!(w >= need_w, "box {w}x{h} must cover the {need_w}px-wide panel");
        assert!((w as f32 / h as f32 - 1080.0 / 1920.0).abs() < 0.01, "aspect must survive: {w}x{h}");
    }

    /// No probe (no webcam, or an unreadable one): a square box, both dims even.
    #[test]
    fn no_source_falls_back_to_a_square_box() {
        let (w, h) = webcam_box(&panels(CamAspect::Square), None, 1440);
        assert_eq!(w, h);
        assert!(w % 2 == 0 && w >= 2);
        assert_eq!(webcam_box(&[], None, 1440), (420, 420));
    }
}

#[cfg(test)]
mod even_tests {
    use super::even_screen;

    #[test]
    fn an_even_capture_is_untouched_and_needs_no_crop() {
        assert_eq!(even_screen(1920, 1080), (1920, 1080, None));
    }

    #[test]
    fn an_odd_capture_loses_one_column_or_row_and_says_so() {
        assert_eq!(even_screen(1697, 955), (1696, 954, Some((1696, 954))));
        assert_eq!(even_screen(1698, 955), (1698, 954, Some((1698, 954))));
        assert_eq!(even_screen(1, 1), (2, 2, Some((2, 2))));
    }
}
