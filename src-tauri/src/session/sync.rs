use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Per-recording timing on the shared capture clock (ms from recording start),
/// so the export can rebuild the real timeline regardless of the labeled fps:
/// each frame is placed by its real capture time, and audio/events are aligned
/// to the same origin.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncLog {
    /// Capture time (ms) of each encoded video frame, in encode order.
    pub frames: Vec<u64>,
    /// Mouse-tracker start (ms); event `t` values are relative to this.
    pub events_ms: u64,
    #[serde(default)]
    pub mic_ms: Option<u64>,
    #[serde(default)]
    pub system_ms: Option<u64>,
}

impl SyncLog {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
    pub fn load(path: &Path) -> io::Result<SyncLog> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trips_through_json() {
        let s = SyncLog { frames: vec![10, 26, 42], events_ms: 5, mic_ms: Some(3), system_ms: None };
        let dir = std::env::temp_dir().join("cursorzoom_synctest.json");
        s.save(&dir).unwrap();
        let back = SyncLog::load(&dir).unwrap();
        assert_eq!(back.frames, vec![10, 26, 42]);
        assert_eq!(back.mic_ms, Some(3));
        assert_eq!(back.system_ms, None);
        let _ = std::fs::remove_file(&dir);
    }
}
