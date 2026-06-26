use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct FramePoint { pub x: i32, pub y: i32 }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Camera { pub cx: f32, pub cy: f32, pub scale: f32 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing { Smooth, Linear, Spring { stiffness: f32, damping: f32 } }

#[derive(Clone, Copy, Debug)]
pub struct ZoomConfig {
    pub target_scale: f32, pub zoom_in_ms: u32, pub zoom_out_ms: u32, pub idle_release_ms: u32,
    pub clicks_to_trigger: u32, pub merge_window_ms: u32, pub merge_radius_px: u32,
    pub follow_damping: f32, pub dead_zone_px: u32, pub easing: Easing,
}
impl Default for ZoomConfig {
    fn default() -> Self {
        Self { target_scale: 1.8, zoom_in_ms: 350, zoom_out_ms: 450, idle_release_ms: 1200,
            clicks_to_trigger: 1, merge_window_ms: 600, merge_radius_px: 240,
            follow_damping: 0.12, dead_zone_px: 60, easing: Easing::Smooth }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZoomRegion {
    pub start_ms: u32, pub end_ms: u32, pub zoom_in_ms: u32, pub zoom_out_ms: u32,
    pub target_scale: f32, pub anchor: FramePoint, pub easing: Easing,
}

#[derive(Clone, Debug)]
pub enum Background { Gradient { from: Rgb, to: Rgb, angle_deg: f32 }, Solid(Rgb), Image(PathBuf) }
impl Default for Background {
    fn default() -> Self {
        Background::Gradient { from: Rgb { r: 36, g: 41, b: 56 }, to: Rgb { r: 88, g: 64, b: 120 }, angle_deg: 135.0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout { pub out_w: u32, pub out_h: u32, pub pad_px: u32 }
impl Default for Layout { fn default() -> Self { Self { out_w: 1920, out_h: 1080, pad_px: 64 } } }

#[derive(Clone, Copy, Debug)] pub enum OverlayShape { Circle, Rounded { radius: u32 }, Rect }
#[derive(Clone, Copy, Debug)] pub enum OverlayPos { BottomLeft, BottomRight, TopLeft, TopRight, Custom { x: u32, y: u32 } }
#[derive(Clone, Copy, Debug)]
pub struct OverlayLayout { pub shape: OverlayShape, pub pos: OverlayPos, pub size_px: u32, pub margin_px: u32, pub enabled: bool }
impl Default for OverlayLayout {
    fn default() -> Self { Self { shape: OverlayShape::Circle, pos: OverlayPos::BottomLeft, size_px: 200, margin_px: 32, enabled: true } }
}
