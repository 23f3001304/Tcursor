use crate::export::settings::Resolution;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePoint {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub cx: f32,
    pub cy: f32,
    pub scale: f32,
}

pub use crate::export::keys::{Key, KeyMode, Keys};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Smooth,
    Linear,
    Spring {
        stiffness: f32,
        damping: f32,
        mass: f32,
    },
    EaseIn,
    EaseOut,
    EaseInOut,
    Cubic {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Keys(Keys),
}

pub const SPRING_DEFAULT: Easing = Easing::Spring {
    stiffness: 100.0,
    damping: 10.0,
    mass: 1.0,
};

#[derive(Clone, Copy, Debug)]
pub struct ZoomConfig {
    pub target_scale: f32,
    pub zoom_in_ms: u32,
    pub zoom_out_ms: u32,
    pub idle_release_ms: u32,
    pub clicks_to_trigger: u32,
    pub merge_window_ms: u32,
    pub merge_radius_px: u32,
    pub follow_damping: f32,
    pub dead_zone_px: u32,
    pub easing: Easing,
    pub smoothing_ms: u32,
}
impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            target_scale: 2.2,
            zoom_in_ms: 350,
            zoom_out_ms: 450,
            idle_release_ms: 2200,
            clicks_to_trigger: 1,
            merge_window_ms: 600,
            merge_radius_px: 240,
            follow_damping: 0.10,
            dead_zone_px: 60,
            easing: Easing::Smooth,
            smoothing_ms: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZoomRegion {
    pub start_ms: u32,
    pub end_ms: u32,
    pub zoom_in_ms: u32,
    pub zoom_out_ms: u32,
    pub target_scale: f32,
    pub anchor: FramePoint,
    pub easing: Easing,
    pub easing_out: Easing,
    pub cam_action: Option<crate::settings::model::CamZoomAction>,
    pub layer: u32,
    pub follow_cursor: bool,
}

#[derive(Clone, Debug)]
pub enum Background {
    Gradient {
        from: Rgb,
        mid: Option<Rgb>,
        to: Rgb,
        angle_deg: f32,
    },
    Solid(Rgb),
    Image(PathBuf),
}
impl Default for Background {
    fn default() -> Self {
        Background::Gradient {
            from: Rgb {
                r: 36,
                g: 41,
                b: 56,
            },
            mid: None,
            to: Rgb {
                r: 88,
                g: 64,
                b: 120,
            },
            angle_deg: 135.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Aspect {
    #[default]
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "wide_16x9")]
    Wide16x9,
    #[serde(rename = "vertical_9x16")]
    Vertical9x16,
    #[serde(rename = "square_1x1")]
    Square1x1,
    #[serde(rename = "classic_4x3")]
    Classic4x3,
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub out_w: u32,
    pub out_h: u32,
    pub pad_px: u32,
    pub screen_scale: f32,
    pub screen_radius_px: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            out_w: 3840,
            out_h: 2160,
            pad_px: 120,
            screen_scale: 1.0,
            screen_radius_px: 2160.0 * 0.016,
        }
    }
}
impl Layout {
    pub fn adapt_to_source(&mut self, sw: u32, sh: u32) {
        if self.out_w == 3840 && self.out_h == 2160 && (sw != 3840 || sh != 2160) {
            self.out_w = sw & !1;
            self.out_h = sh & !1;
        }
    }

    pub fn apply_aspect(&mut self, aspect: Aspect, sw: u32, sh: u32) {
        match aspect {
            Aspect::Source => self.adapt_to_source(sw, sh),
            Aspect::Wide16x9 => {
                self.out_w = 1920;
                self.out_h = 1080;
            }
            Aspect::Vertical9x16 => {
                self.out_w = 1080;
                self.out_h = 1920;
            }
            Aspect::Square1x1 => {
                self.out_w = 1080;
                self.out_h = 1080;
            }
            Aspect::Classic4x3 => {
                self.out_w = 1440;
                self.out_h = 1080;
            }
        }
    }

    pub fn scale_to_long_edge(w: u32, h: u32, max_long: u32) -> (u32, u32) {
        let long = w.max(h).max(1) as f32;
        let k = (max_long as f32 / long).min(1.0);
        (
            ((w as f32 * k).round() as u32 & !1).max(2),
            ((h as f32 * k).round() as u32 & !1).max(2),
        )
    }

    pub fn resolve(
        &mut self,
        aspect: Aspect,
        resolution: Resolution,
        sw: u32,
        sh: u32,
        preview_cap: Option<u32>,
    ) {
        self.apply_aspect(aspect, sw, sh);
        self.rescale_to_resolution(resolution);
        if let Some(cap) = preview_cap {
            let (pw, ph) = Self::scale_to_long_edge(self.out_w, self.out_h, cap);
            let s = pw as f32 / self.out_w.max(1) as f32;
            self.pad_px = (self.pad_px as f32 * s).round() as u32;
            self.screen_radius_px *= s;
            self.out_w = pw;
            self.out_h = ph;
        }
    }
}

pub use crate::export::overlay::{OverlayLayout, OverlayPos, OverlayShape};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_default_is_source() {
        assert_eq!(Aspect::default(), Aspect::Source);
    }

    #[test]
    fn source_aspect_matches_adapt_to_source_exactly() {
        let mut a = Layout::default();
        a.apply_aspect(Aspect::Source, 1920, 1080);
        let mut b = Layout::default();
        b.adapt_to_source(1920, 1080);
        assert_eq!((a.out_w, a.out_h), (b.out_w, b.out_h));
        assert_eq!((a.out_w, a.out_h), (1920, 1080));
    }

    #[test]
    fn fixed_presets_map_to_a_1920_long_edge() {
        let mut l = Layout::default();
        l.apply_aspect(Aspect::Wide16x9, 640, 480);
        assert_eq!((l.out_w, l.out_h), (1920, 1080));
        l.apply_aspect(Aspect::Vertical9x16, 640, 480);
        assert_eq!((l.out_w, l.out_h), (1080, 1920));
        l.apply_aspect(Aspect::Square1x1, 640, 480);
        assert_eq!((l.out_w, l.out_h), (1080, 1080));
        l.apply_aspect(Aspect::Classic4x3, 640, 480);
        assert_eq!((l.out_w, l.out_h), (1440, 1080));
    }

    #[test]
    fn scale_to_long_edge_preserves_ratio_and_evenizes() {
        assert_eq!(Layout::scale_to_long_edge(1920, 1080, 1280), (1280, 720));
        assert_eq!(Layout::scale_to_long_edge(1080, 1920, 1280), (720, 1280));
        assert_eq!(Layout::scale_to_long_edge(640, 480, 1280), (640, 480));
    }

    #[test]
    fn resolve_scales_pad_and_radius_with_the_preview_cap() {
        let mut l = Layout::default();
        l.resolve(Aspect::Wide16x9, Resolution::Source, 640, 480, Some(1280));
        assert_eq!((l.out_w, l.out_h), (1280, 720));
        assert_eq!(l.pad_px, 80);
    }
}
