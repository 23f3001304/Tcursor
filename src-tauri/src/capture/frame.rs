use crate::domain::time::Timestamp;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub ts: Timestamp,
}
