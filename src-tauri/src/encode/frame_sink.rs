use crate::capture::frame::Frame;

pub trait FrameSink: Send {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool>;
    fn finish(self: Box<Self>) -> std::io::Result<()>;

    fn split(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeFrameSink {
    pub pushed: Vec<(u32, u32)>,
}

impl FrameSink for FakeFrameSink {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool> {
        self.pushed.push((f.width, f.height));
        Ok(true)
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::frame::Frame;
    use crate::domain::time::Timestamp;

    #[test]
    fn fake_sink_records_pushes_and_finish() {
        let mut sink = FakeFrameSink::default();
        let f = Frame {
            width: 4,
            height: 2,
            bgra: vec![0; 4 * 2 * 4],
            ts: Timestamp(0),
        };
        assert!(sink.push(&f).unwrap());
        assert_eq!(sink.pushed, vec![(4, 2)]);
        Box::new(sink).finish().unwrap();
    }
}
