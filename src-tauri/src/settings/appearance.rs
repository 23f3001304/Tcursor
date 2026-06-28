use serde::{Deserialize, Serialize};
use crate::actions::model::LayoutId;
use crate::export::types::{Layout, OverlayLayout, OverlayPos, OverlayShape};

/// Webcam bubble shape. `Rounded` uses `cam_radius` (fraction of the panel's
/// min side); `Circle` is a full pill; `Rect` has square corners.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CamShape { Circle, Rounded, Rect }

/// Which corner the webcam bubble anchors to (bubble modes only).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CamCorner { BottomLeft, BottomRight, TopLeft, TopRight }

/// Per-mode appearance. Sizes are fractions of the canvas (resolution-
/// independent); the render maps them to pixels at resolve time. Defaults
/// reproduce today's hardcoded render exactly (see `Default`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ModeAppearance {
    pub pad: f32,           // frac of canvas WIDTH  -> inset padding (both axes)
    pub screen_size: f32,   // 0.6..1.0 scale of the screen panel about its center
    pub screen_radius: f32, // frac of canvas HEIGHT -> screen corner radius
    pub cam_size: f32,      // frac of canvas HEIGHT -> camera panel height (square)
    pub cam_shape: CamShape,
    pub cam_radius: f32,    // frac of panel min-side, used only when Rounded
    pub cam_corner: CamCorner,
    pub cam_margin_x: f32,  // frac of canvas WIDTH  (bubble modes)
    pub cam_margin_y: f32,  // frac of canvas HEIGHT (bubble modes)
}
impl Default for ModeAppearance {
    fn default() -> Self {
        Self { pad: 0.03125, screen_size: 1.0, screen_radius: 0.016, cam_size: 0.1944,
            cam_shape: CamShape::Circle, cam_radius: 0.04, cam_corner: CamCorner::BottomLeft,
            cam_margin_x: 0.0208, cam_margin_y: 0.037 }
    }
}

/// Appearance for all five layout modes. Defaults differ per mode (the big-camera
/// modes use a large rounded square; the bubble modes use the small circle).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub screen: ModeAppearance,
    pub screen_only: ModeAppearance,
    pub camera: ModeAppearance,
    pub camera_only: ModeAppearance,
    pub presenter: ModeAppearance,
}
impl Default for AppearanceSettings {
    fn default() -> Self {
        let bubble = ModeAppearance::default();
        let big = ModeAppearance { cam_size: 0.889, cam_shape: CamShape::Rounded, ..bubble };
        Self { screen: bubble, screen_only: bubble, camera: big, camera_only: big, presenter: big }
    }
}
impl AppearanceSettings {
    /// The appearance block for a layout mode.
    pub fn for_id(&self, id: LayoutId) -> &ModeAppearance {
        match id {
            LayoutId::Screen => &self.screen,
            LayoutId::ScreenOnly => &self.screen_only,
            LayoutId::Camera => &self.camera,
            LayoutId::CameraOnly => &self.camera_only,
            LayoutId::Presenter => &self.presenter,
        }
    }
}

/// Build the per-mode `Layout`: canvas dims + padding + screen scale/radius (px).
pub fn layout_for(ma: &ModeAppearance, ow: u32, oh: u32) -> Layout {
    Layout {
        out_w: ow, out_h: oh,
        pad_px: (ma.pad * ow as f32).round() as u32,
        screen_scale: ma.screen_size,
        screen_radius_px: ma.screen_radius * oh as f32,
    }
}

/// Build the per-mode webcam `OverlayLayout`: shape/pos/size/margins in px.
pub fn overlay_for(ma: &ModeAppearance, ow: u32, oh: u32, enabled: bool) -> OverlayLayout {
    let shape = match ma.cam_shape {
        CamShape::Circle => OverlayShape::Circle,
        CamShape::Rounded => OverlayShape::Rounded { frac: ma.cam_radius },
        CamShape::Rect => OverlayShape::Rect,
    };
    let pos = match ma.cam_corner {
        CamCorner::BottomLeft => OverlayPos::BottomLeft,
        CamCorner::BottomRight => OverlayPos::BottomRight,
        CamCorner::TopLeft => OverlayPos::TopLeft,
        CamCorner::TopRight => OverlayPos::TopRight,
    };
    OverlayLayout {
        shape, pos,
        size_px: (ma.cam_size * oh as f32).round() as u32,
        margin_x_px: (ma.cam_margin_x * ow as f32).round() as u32,
        margin_y_px: (ma.cam_margin_y * oh as f32).round() as u32,
        enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_reproduce_today_values() {
        let a = AppearanceSettings::default();
        assert_eq!(a.screen.pad, 0.03125);              // 120px at 3840 wide
        assert_eq!(a.screen.cam_shape, CamShape::Circle);
        assert_eq!(a.screen.cam_corner, CamCorner::BottomLeft);
        assert_eq!(a.camera.cam_size, 0.889);           // 1920px at 2160 tall
        assert_eq!(a.camera.cam_shape, CamShape::Rounded);
    }
    #[test]
    fn for_id_maps_each_mode() {
        let a = AppearanceSettings::default();
        assert_eq!(*a.for_id(LayoutId::Screen), a.screen);
        assert_eq!(*a.for_id(LayoutId::ScreenOnly), a.screen_only);
        assert_eq!(*a.for_id(LayoutId::Camera), a.camera);
        assert_eq!(*a.for_id(LayoutId::CameraOnly), a.camera_only);
        assert_eq!(*a.for_id(LayoutId::Presenter), a.presenter);
    }
    #[test]
    fn round_trip_and_partial_json_fill_defaults() {
        let a = AppearanceSettings::default();
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"cam_shape\":\"circle\""));
        assert!(json.contains("\"cam_corner\":\"bottom_left\""));
        let back: AppearanceSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
        // partial per-mode JSON fills missing fields from ModeAppearance::default
        let p: AppearanceSettings = serde_json::from_str("{\"screen\":{\"pad\":0.05}}").unwrap();
        assert_eq!(p.screen.pad, 0.05);
        assert_eq!(p.screen.cam_size, ModeAppearance::default().cam_size);
        assert_eq!(p.camera, AppearanceSettings::default().camera);
    }
    #[test]
    fn layout_for_default_screen_is_today_px() {
        let l = layout_for(&AppearanceSettings::default().screen, 3840, 2160);
        assert_eq!(l.pad_px, 120);
        assert_eq!(l.screen_scale, 1.0);
        assert!((l.screen_radius_px - 34.56).abs() < 0.01);
    }
    #[test]
    fn overlay_for_default_screen_is_today_bubble() {
        use crate::export::types::{OverlayPos, OverlayShape};
        let o = overlay_for(&AppearanceSettings::default().screen, 3840, 2160, true);
        assert_eq!(o.size_px, 420);
        assert_eq!((o.margin_x_px, o.margin_y_px), (80, 80));
        assert!(matches!(o.shape, OverlayShape::Circle));
        assert!(matches!(o.pos, OverlayPos::BottomLeft));
    }
    #[test]
    fn overlay_for_default_camera_is_rounded_1920() {
        use crate::export::types::OverlayShape;
        let o = overlay_for(&AppearanceSettings::default().camera, 3840, 2160, true);
        assert_eq!(o.size_px, 1920);          // 0.889 * 2160
        assert!(matches!(o.shape, OverlayShape::Rounded { .. }));
    }
}
