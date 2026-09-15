#[derive(Clone, Copy, Debug)]
pub enum OverlayShape {
    Circle,
    Rounded { frac: f32 },
    Rect,
}
#[derive(Clone, Copy, Debug)]
pub enum OverlayPos {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
    Custom { x: u32, y: u32 },
}
#[derive(Clone, Copy, Debug)]
pub struct OverlayLayout {
    pub shape: OverlayShape,
    pub pos: OverlayPos,
    pub size_px: u32,
    pub width_px: u32,
    pub margin_x_px: u32,
    pub margin_y_px: u32,
    pub enabled: bool,
    pub ring_px: u32,
    pub ring_color: [u8; 3],
}
impl Default for OverlayLayout {
    fn default() -> Self {
        Self {
            shape: OverlayShape::Circle,
            pos: OverlayPos::BottomLeft,
            size_px: 420,
            width_px: 420,
            margin_x_px: 80,
            margin_y_px: 80,
            enabled: true,
            ring_px: 0,
            ring_color: [0, 0, 0],
        }
    }
}
