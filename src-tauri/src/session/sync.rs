use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Per-recording timing on the shared capture clock (ms from recording start),
/// so the export can rebuild the real timeline regardless of the labeled fps:
/// each frame is placed by its real capture time, and audio/events are aligned
/// to the same origin.
/// One extra input segment recorded after a mid-take source switch: the file (relative to the
/// project folder) and where it starts on the recording clock. The FIRST segment is never listed
/// here - `mic.wav` at `mic_ms`, `webcam.webm` at its own start - so a take with no switches has
/// empty lists and every reader that predates switching is unaffected.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Segment { pub path: String, pub start_ms: u64 }

/// A mid-take display (or window) switch: when it happened on the recording clock, what was
/// switched to, and the NEW capture's own frame size. The capture keeps feeding the same encoder
/// canvas, so from `at_ms` on every frame is `frame_fit::letterbox((w, h), canvas)` - the render
/// crops that fitted rect back out so the baked black bars never reach the screen panel.
/// `w`/`h` are `#[serde(default)]`: a log written before this existed reads `0`, which every
/// reader treats as "unknown size" = the full canvas, i.e. exactly the pre-crop behavior.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DisplaySwitch {
    pub at_ms: u64,
    pub target_id: String,
    #[serde(default)] pub w: u32,
    #[serde(default)] pub h: u32,
}

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
    /// Mid-take source switching (2026-09-14): the extra mic and webcam segments `preprocess`
    /// merges into the single files, and the display switches. All empty for an ordinary take.
    #[serde(default)]
    pub mic_segments: Vec<Segment>,
    #[serde(default)]
    pub webcam_segments: Vec<Segment>,
    #[serde(default)]
    pub display_switches: Vec<DisplaySwitch>,
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
        let s = SyncLog { frames: vec![10, 26, 42], events_ms: 5, mic_ms: Some(3), system_ms: None,
            mic_segments: vec![Segment { path: "mic_2.wav".into(), start_ms: 4200 }], webcam_segments: Vec::new(),
            display_switches: vec![DisplaySwitch { at_ms: 9000, target_id: "display:1".into(), w: 1280, h: 800 }] };
        let dir = std::env::temp_dir().join("cursorzoom_synctest.json");
        s.save(&dir).unwrap();
        let back = SyncLog::load(&dir).unwrap();
        assert_eq!(back.frames, vec![10, 26, 42]);
        assert_eq!(back.mic_ms, Some(3));
        assert_eq!(back.system_ms, None);
        assert_eq!(back.mic_segments, vec![Segment { path: "mic_2.wav".into(), start_ms: 4200 }]);
        assert_eq!(back.display_switches[0].target_id, "display:1");
        assert_eq!((back.display_switches[0].w, back.display_switches[0].h), (1280, 800));
        let _ = std::fs::remove_file(&dir);
    }

    /// A switch logged before the geometry was recorded reads `0x0` - the "unknown size" the span
    /// table turns into a full-canvas span, so such a take renders exactly as it did before.
    #[test]
    fn a_switch_without_geometry_reads_zero() {
        let s: SyncLog = serde_json::from_str(
            r#"{"frames":[1],"events_ms":0,"display_switches":[{"at_ms":900,"target_id":"display:2"}]}"#).unwrap();
        assert_eq!((s.display_switches[0].w, s.display_switches[0].h), (0, 0));
    }

    /// A sync.json written before switching existed has none of the three lists; they read empty.
    #[test]
    fn an_older_log_without_segments_still_loads() {
        let s: SyncLog = serde_json::from_str(r#"{"frames":[1,2],"events_ms":0,"mic_ms":7}"#).unwrap();
        assert!(s.mic_segments.is_empty() && s.webcam_segments.is_empty() && s.display_switches.is_empty());
    }
}
