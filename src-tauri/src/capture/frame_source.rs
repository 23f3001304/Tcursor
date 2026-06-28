use crate::capture::frame::Frame;

pub trait FrameSource: Send {
    fn dimensions(&self) -> (u32, u32);
    /// Returns the next frame, or None when the source has ended.
    fn next_frame(&mut self) -> Option<Frame>;
    /// Newest frame available right now, discarding any older buffered frames;
    /// None if none is ready. Non-blocking. Used by fixed-rate (game) pacing.
    fn drain_latest(&mut self) -> Option<Frame>;
}

pub struct FakeFrameSource {
    frames: std::collections::VecDeque<Frame>,
    dims: (u32, u32),
}

impl FakeFrameSource {
    pub fn new(frames: Vec<Frame>) -> Self {
        let dims = frames.first().map(|f| (f.width, f.height)).unwrap_or((0, 0));
        Self { frames: frames.into(), dims }
    }
}

impl FrameSource for FakeFrameSource {
    fn dimensions(&self) -> (u32, u32) { self.dims }
    fn next_frame(&mut self) -> Option<Frame> { self.frames.pop_front() }
    fn drain_latest(&mut self) -> Option<Frame> {
        let mut last = None;
        while let Some(f) = self.frames.pop_front() { last = Some(f); }
        last
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::frame::Frame;
    use crate::domain::time::Timestamp;

    fn frame(ts: u64) -> Frame {
        Frame { width: 2, height: 1, bgra: vec![0,0,0,255, 1,1,1,255], ts: Timestamp(ts) }
    }

    #[test]
    fn fake_source_drains_in_order_then_ends() {
        let mut src = FakeFrameSource::new(vec![frame(0), frame(33)]);
        assert_eq!(src.dimensions(), (2,1));
        assert_eq!(src.next_frame().unwrap().ts, Timestamp(0));
        assert_eq!(src.next_frame().unwrap().ts, Timestamp(33));
        assert!(src.next_frame().is_none());
    }

    #[test]
    fn drain_latest_returns_newest_and_empties() {
        let mut src = FakeFrameSource::new(vec![frame(0), frame(16), frame(33)]);
        assert_eq!(src.drain_latest().unwrap().ts, Timestamp(33)); // newest
        assert!(src.drain_latest().is_none());                     // drained
    }
}
