use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Segment {
    pub path: String,
    pub start_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DisplaySwitch {
    pub at_ms: u64,
    pub target_id: String,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncLog {
    pub frames: Vec<u64>,
    pub events_ms: u64,
    #[serde(default)]
    pub mic_ms: Option<u64>,
    #[serde(default)]
    pub system_ms: Option<u64>,
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
        let s = SyncLog {
            frames: vec![10, 26, 42],
            events_ms: 5,
            mic_ms: Some(3),
            system_ms: None,
            mic_segments: vec![Segment {
                path: "mic_2.wav".into(),
                start_ms: 4200,
            }],
            webcam_segments: Vec::new(),
            display_switches: vec![DisplaySwitch {
                at_ms: 9000,
                target_id: "display:1".into(),
                w: 1280,
                h: 800,
            }],
        };
        let dir = std::env::temp_dir().join("cursorzoom_synctest.json");
        s.save(&dir).unwrap();
        let back = SyncLog::load(&dir).unwrap();
        assert_eq!(back.frames, vec![10, 26, 42]);
        assert_eq!(back.mic_ms, Some(3));
        assert_eq!(back.system_ms, None);
        assert_eq!(
            back.mic_segments,
            vec![Segment {
                path: "mic_2.wav".into(),
                start_ms: 4200
            }]
        );
        assert_eq!(back.display_switches[0].target_id, "display:1");
        assert_eq!(
            (back.display_switches[0].w, back.display_switches[0].h),
            (1280, 800)
        );
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn a_switch_without_geometry_reads_zero() {
        let s: SyncLog = serde_json::from_str(
            r#"{"frames":[1],"events_ms":0,"display_switches":[{"at_ms":900,"target_id":"display:2"}]}"#).unwrap();
        assert_eq!((s.display_switches[0].w, s.display_switches[0].h), (0, 0));
    }

    #[test]
    fn an_older_log_without_segments_still_loads() {
        let s: SyncLog =
            serde_json::from_str(r#"{"frames":[1,2],"events_ms":0,"mic_ms":7}"#).unwrap();
        assert!(
            s.mic_segments.is_empty()
                && s.webcam_segments.is_empty()
                && s.display_switches.is_empty()
        );
    }
}
