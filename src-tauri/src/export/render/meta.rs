use crate::export::pipeline::timeline::Timeline;
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint};

pub struct RenderMeta {
    pub tl: Timeline,
    pub video_start: u64,
    pub video_end: u64,
    pub out_w: u32,
    pub out_h: u32,
    pub sw: u32,
    pub sh: u32,
    pub screen_bytes: usize,
    pub audio_offset_ms: i32,
    pub screen_crop: Option<(u32, u32)>,
    pub webcam_w: u32,
    pub webcam_h: u32,
    pub trim: crate::edit::model::Trim,
    pub mic_volume: f32,
    pub sys_volume: f32,
}

pub fn even_screen(w: u32, h: u32) -> (u32, u32, Option<(u32, u32)>) {
    let (ew, eh) = ((w & !1).max(2), (h & !1).max(2));
    (ew, eh, ((ew, eh) != (w, h)).then_some((ew, eh)))
}

pub struct FramePose {
    pub ev_t: u32,
    pub out_t: u32,
    pub scene: Scene,
    pub cur: FramePoint,
    pub cam: Camera,
    pub mix: Option<crate::export::render::spans::SpanMix>,
    pub hold: Option<usize>,
}

pub fn webcam_box(
    panels: &[crate::export::types::OverlayLayout],
    src: Option<(u32, u32)>,
    cap: u32,
) -> (u32, u32) {
    let need_w = panels
        .iter()
        .map(|o| o.width_px.max(o.size_px))
        .max()
        .unwrap_or(420)
        .max(2);
    let need_h = panels.iter().map(|o| o.size_px).max().unwrap_or(420).max(2);
    let (sw, sh) = src
        .filter(|(w, h)| *w >= 2 && *h >= 2)
        .unwrap_or((cap, cap));
    let aspect = sw as f32 / sh as f32;
    let h = need_h
        .max((need_w as f32 / aspect).ceil() as u32)
        .min(cap)
        .min(sh);
    let even = |v: u32| (v & !1).max(2);
    (even((h as f32 * aspect).round() as u32), even(h))
}

#[cfg(test)]
mod tests {
    use super::webcam_box;
    use crate::export::types::OverlayLayout;
    use crate::settings::appearance::{overlay_for, AppearanceSettings, CamAspect};

    fn panels_at(aspect: CamAspect, oh: u32) -> Vec<OverlayLayout> {
        let mut a = AppearanceSettings::default();
        a.screen.cam_aspect = aspect;
        a.screen_only.cam_aspect = aspect;
        [
            &a.screen,
            &a.screen_only,
            &a.camera,
            &a.camera_only,
            &a.presenter,
        ]
        .iter()
        .map(|m| overlay_for(m, oh * 16 / 9, oh, true))
        .collect()
    }
    fn panels(aspect: CamAspect) -> Vec<OverlayLayout> {
        panels_at(aspect, 1080)
    }

    #[test]
    fn the_box_takes_the_sources_aspect_not_a_panels() {
        for aspect in [CamAspect::Square, CamAspect::Wide] {
            let (w, h) = webcam_box(&panels(aspect), Some((1280, 720)), 1440);
            assert_eq!((w, h), (1280, 720), "{aspect:?}");
        }
        let (w, h) = webcam_box(&panels(CamAspect::Square), Some((640, 480)), 1440);
        assert_eq!((w, h), (640, 480));
    }

    #[test]
    fn the_box_is_clamped_by_the_source_and_the_cap() {
        assert_eq!(
            webcam_box(
                &panels_at(CamAspect::Square, 2160),
                Some((3840, 2160)),
                1440
            )
            .1,
            1440
        );
        assert_eq!(
            webcam_box(&panels(CamAspect::Square), Some((320, 240)), 1440),
            (320, 240)
        );
    }

    #[test]
    fn the_box_covers_the_widest_panel_on_a_tall_source() {
        let p = panels(CamAspect::Wide);
        let need_w = p.iter().map(|o| o.width_px.max(o.size_px)).max().unwrap();
        let (w, h) = webcam_box(&p, Some((1080, 1920)), 4096);
        assert!(
            w >= need_w,
            "box {w}x{h} must cover the {need_w}px-wide panel"
        );
        assert!(
            (w as f32 / h as f32 - 1080.0 / 1920.0).abs() < 0.01,
            "aspect must survive: {w}x{h}"
        );
    }

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
