use crate::events::model::ScreenInfo;
use crate::export::types::FramePoint;

pub fn to_frame(screen: &ScreenInfo, x: i32, y: i32) -> FramePoint {
    FramePoint { x: x - screen.origin_x, y: y - screen.origin_y }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_screen_to_frame_minus_origin() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 100, origin_y: 50 };
        let p = to_frame(&s, 300, 200);
        assert_eq!((p.x, p.y), (200, 150));
    }
}
