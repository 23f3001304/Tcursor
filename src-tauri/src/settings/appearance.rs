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

/// Webcam panel aspect ratio. `Square` (1:1) is today's only shape; `Wide` (16:9)
/// widens the panel while keeping `cam_size` as the HEIGHT basis.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CamAspect { Square, Wide }

/// Optional colored ring/border drawn just inside the webcam panel edge.
/// `width` is a fraction of the panel's min side (so it scales with `cam_size`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct CamRing { pub width: f32, pub color: [u8; 3] }

/// Per-mode appearance. Sizes are fractions of the canvas (resolution-
/// independent); the render maps them to pixels at resolve time. Defaults
/// reproduce today's hardcoded render exactly (see `Default`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ModeAppearance {
    pub pad: f32,           // frac of canvas WIDTH  -> inset padding (both axes)
    pub screen_size: f32,   // 0.6..1.0 scale of the screen panel about its center
    pub screen_radius: f32, // frac of canvas HEIGHT -> screen corner radius
    pub cam_size: f32,      // frac of canvas HEIGHT -> camera panel height
    pub cam_shape: CamShape,
    pub cam_radius: f32,    // frac of panel min-side, used only when Rounded
    pub cam_corner: CamCorner,
    pub cam_margin_x: f32,  // frac of canvas WIDTH  (bubble modes)
    pub cam_margin_y: f32,  // frac of canvas HEIGHT (bubble modes)
    pub cam_aspect: CamAspect, // panel width:height; Square = today's behavior
    #[serde(default)] pub cam_ring: Option<CamRing>, // None = no ring (today's behavior)
}
impl Default for ModeAppearance {
    fn default() -> Self {
        Self { pad: 0.03125, screen_size: 1.0, screen_radius: 0.016, cam_size: 0.1944,
            cam_shape: CamShape::Circle, cam_radius: 0.04, cam_corner: CamCorner::BottomLeft,
            cam_margin_x: 0.0208, cam_margin_y: 0.037, cam_aspect: CamAspect::Square, cam_ring: None }
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
    let size_px = (ma.cam_size * oh as f32).round() as u32;
    let width_px = match ma.cam_aspect {
        CamAspect::Square => size_px,
        CamAspect::Wide => (size_px as f32 * 16.0 / 9.0).round() as u32,
    };
    let (ring_px, ring_color) = match ma.cam_ring {
        Some(r) => ((r.width * width_px.min(size_px) as f32).round() as u32, r.color),
        None => (0, [0, 0, 0]),
    };
    OverlayLayout {
        shape, pos,
        size_px, width_px,
        margin_x_px: (ma.cam_margin_x * ow as f32).round() as u32,
        margin_y_px: (ma.cam_margin_y * oh as f32).round() as u32,
        enabled, ring_px, ring_color,
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
