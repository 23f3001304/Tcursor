#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameIndex(pub u64);
impl FrameIndex { pub fn next(self) -> FrameIndex { FrameIndex(self.0 + 1) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenCoord { pub x: i32, pub y: i32 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasCoord { pub x: f32, pub y: f32 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_index_increments() {
        assert_eq!(FrameIndex(0).next(), FrameIndex(1));
    }
    #[test]
    fn screen_and_canvas_coords_are_distinct_types() {
        let s = ScreenCoord { x: 10, y: 20 };
        let c = CanvasCoord { x: 1.0, y: 2.0 };
        assert_eq!(s.x, 10);
        assert_eq!(c.x, 1.0);
    }
}
