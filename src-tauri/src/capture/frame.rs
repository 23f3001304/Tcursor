use crate::domain::time::Timestamp;

/// One captured frame in BGRA8 (row-major, 4 bytes/pixel).
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub ts: Timestamp,
}
