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
    pub webcam_w: u32, pub webcam_h: u32, // webcam decode box: the widest panel's own (w, h)
    pub trim: crate::edit::model::Trim, // unresolved (`Trim::resolve`); avoids a 2nd doc load in `exporter::export`
    pub mic_volume: f32, pub sys_volume: f32, // 0..1.5 gain per track at mux (`audio_mux::mux`)
}

/// Camera + scene for one output frame, carrying BOTH clocks: `ev_t` indexes the raw event streams
/// (cursor/mouse/actions), `out_t` is the timeline every `EditDoc` region list lives on.
pub struct FramePose { pub ev_t: u32, pub out_t: u32, pub scene: Scene, pub cur: FramePoint, pub cam: Camera }

/// The webcam decode box for the largest resolved camera panel: its own `(width_px, size_px)`
/// aspect, capped at `cap` tall and rounded to even dims for the decoder.
pub fn webcam_dims(ov: &crate::export::types::OverlayLayout, cap: u32) -> (u32, u32) {
    let aspect = ov.width_px.max(1) as f32 / ov.size_px.max(1) as f32;
    let h = ov.size_px.clamp(2, cap);
    let even = |v: u32| (v & !1).max(2);
    (even((h as f32 * aspect).round() as u32), even(h))
}

#[cfg(test)]
mod tests {
    use super::webcam_dims;
    use crate::settings::appearance::{overlay_for, AppearanceSettings, CamAspect};

    fn dims(aspect: CamAspect, cam_size: f32, oh: u32, cap: u32) -> (u32, u32) {
        let mut a = AppearanceSettings::default();
        a.screen.cam_aspect = aspect;
        a.screen.cam_size = cam_size;
        webcam_dims(&overlay_for(&a.screen, oh * 16 / 9, oh, true), cap)
    }

    /// A Wide panel decodes 16:9, not a square the compositor then stretches 1.78x.
    #[test]
    fn wide_panel_decodes_at_sixteen_by_nine() {
        let (w, h) = dims(CamAspect::Wide, 252.0 / 1080.0, 1080, 1440);
        assert_eq!((w, h), (448, 252));
        assert!(((w as f32 / h as f32) - 16.0 / 9.0).abs() < 0.02);
    }

    /// A Square panel is unchanged, and both dims stay even for the decoder.
    #[test]
    fn square_panel_stays_square_and_even() {
        assert_eq!(dims(CamAspect::Square, 0.25, 1080, 1440), (270, 270));
        assert_eq!(dims(CamAspect::Square, 269.0 / 1080.0, 1080, 1440), (268, 268));
    }

    /// The cap bounds HEIGHT; width re-derives from the aspect so the box never skews.
    #[test]
    fn cap_bounds_height_without_skewing_the_aspect() {
        let (w, h) = dims(CamAspect::Wide, 0.9, 2160, 1440);
        assert_eq!(h, 1440);
        assert!(((w as f32 / h as f32) - 16.0 / 9.0).abs() < 0.02, "got {w}x{h}");
    }
}
