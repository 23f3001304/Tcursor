#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureSource {
    Display(u32),
    Window(u64),
    Region { x: i32, y: i32, w: u32, h: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_holds_rect() {
        let r = CaptureSource::Region { x: 0, y: 0, w: 800, h: 600 };
        match r { CaptureSource::Region { w, .. } => assert_eq!(w, 800), _ => panic!() }
    }
}
