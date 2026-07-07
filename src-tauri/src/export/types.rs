use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct FramePoint { pub x: i32, pub y: i32 }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct RectF { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Camera { pub cx: f32, pub cy: f32, pub scale: f32 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing { Smooth, Linear, Spring { stiffness: f32, damping: f32 }, EaseIn, EaseOut, EaseInOut }

#[derive(Clone, Copy, Debug)]
pub struct ZoomConfig {
    pub target_scale: f32, pub zoom_in_ms: u32, pub zoom_out_ms: u32, pub idle_release_ms: u32,
    pub clicks_to_trigger: u32, pub merge_window_ms: u32, pub merge_radius_px: u32,
    pub follow_damping: f32, pub dead_zone_px: u32, pub easing: Easing,
}
impl Default for ZoomConfig {
    fn default() -> Self {
        Self { target_scale: 2.2, zoom_in_ms: 350, zoom_out_ms: 450, idle_release_ms: 2200,
            clicks_to_trigger: 1, merge_window_ms: 600, merge_radius_px: 240,
            follow_damping: 0.10, dead_zone_px: 60, easing: Easing::Smooth }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZoomRegion {
    pub start_ms: u32, pub end_ms: u32, pub zoom_in_ms: u32, pub zoom_out_ms: u32,
    pub target_scale: f32, pub anchor: FramePoint, pub easing: Easing,
    /// Priority when this region overlaps another - higher wins (see `CameraSim::step`).
    pub layer: u32,
}

#[derive(Clone, Debug)]
pub enum Background { Gradient { from: Rgb, to: Rgb, angle_deg: f32 }, Solid(Rgb), Image(PathBuf) }
impl Default for Background {
    fn default() -> Self {
        Background::Gradient { from: Rgb { r: 36, g: 41, b: 56 }, to: Rgb { r: 88, g: 64, b: 120 }, angle_deg: 135.0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout { pub out_w: u32, pub out_h: u32, pub pad_px: u32, pub screen_scale: f32, pub screen_radius_px: f32 }
impl Default for Layout { fn default() -> Self { Self { out_w: 3840, out_h: 2160, pad_px: 120, screen_scale: 1.0, screen_radius_px: 2160.0 * 0.016 } } }

#[derive(Clone, Copy, Debug)] pub enum OverlayShape { Circle, Rounded { frac: f32 }, Rect }
#[derive(Clone, Copy, Debug)] pub enum OverlayPos { BottomLeft, BottomRight, TopLeft, TopRight, Custom { x: u32, y: u32 } }
#[derive(Clone, Copy, Debug)]
pub struct OverlayLayout {
    pub shape: OverlayShape, pub pos: OverlayPos,
    pub size_px: u32,   // camera panel HEIGHT
    pub width_px: u32,  // camera panel WIDTH (== size_px unless cam_aspect is Wide)
    pub margin_x_px: u32, pub margin_y_px: u32, pub enabled: bool,
    pub ring_px: u32,        // ring/border width in px, 0 = no ring
    pub ring_color: [u8; 3],
}
impl Default for OverlayLayout {
    fn default() -> Self {
        Self { shape: OverlayShape::Circle, pos: OverlayPos::BottomLeft, size_px: 420, width_px: 420,
            margin_x_px: 80, margin_y_px: 80, enabled: true, ring_px: 0, ring_color: [0, 0, 0] }
    }
}
