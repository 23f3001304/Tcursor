use serde::{Deserialize, Serialize};
use std::path::Path;

/// The OS cursor shape at a given moment.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CursorType {
    Arrow,
    IBeam,
    Hand,
    #[serde(rename = "resize_ns")]
    ResizeNs,
    #[serde(rename = "resize_ew")]
    ResizeEw,
    #[serde(rename = "resize_nwse")]
    ResizeNwse,
    #[serde(rename = "resize_nesw")]
    ResizeNesw,
    Move,
    Busy,
}

impl Default for CursorType {
    fn default() -> Self { CursorType::Arrow }
}

/// Monotonic timeline of cursor-type changes. One entry per shape-change event.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CursorTrack {
    pub samples: Vec<(u32, CursorType)>,
}

impl CursorTrack {
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    /// Returns an empty track on any error (missing file, parse failure, etc.).
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Returns the cursor type active at `t_ms` (last sample with `sample.0 <= t_ms`),
    /// or `Arrow` if the track is empty or all samples are after `t_ms`.
    pub fn type_at(&self, t_ms: u32) -> CursorType {
        let idx = self.samples.partition_point(|&(t, _)| t <= t_ms);
        if idx == 0 {
            CursorType::Arrow
        } else {
            self.samples[idx - 1].1
        }
    }
}

// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_at_empty_returns_arrow() {
        let track = CursorTrack::default();
        assert_eq!(track.type_at(0), CursorType::Arrow);
        assert_eq!(track.type_at(9999), CursorType::Arrow);
    }

    #[test]
    fn type_at_lookups() {
        let track = CursorTrack {
            samples: vec![
                (0, CursorType::Arrow),
                (100, CursorType::IBeam),
                (250, CursorType::Hand),
            ],
        };
        // t=50: only (0,Arrow) applies
        assert_eq!(track.type_at(50), CursorType::Arrow);
        // t=100: exactly on the IBeam boundary
        assert_eq!(track.type_at(100), CursorType::IBeam);
        // t=200: still IBeam (Hand hasn't started yet)
        assert_eq!(track.type_at(200), CursorType::IBeam);
        // t=9999: Hand
        assert_eq!(track.type_at(9999), CursorType::Hand);
    }

    #[test]
    fn type_at_before_first_sample_returns_arrow() {
        // All samples start at t=50, so t=10 should return Arrow.
        let track = CursorTrack {
            samples: vec![(50, CursorType::Hand)],
        };
        assert_eq!(track.type_at(10), CursorType::Arrow);
    }

    #[test]
    fn round_trip_save_load() {
        let dir = std::env::temp_dir().join("tcursor_cursortype_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cursor.json");
        let track = CursorTrack {
            samples: vec![
                (0, CursorType::Arrow),
                (100, CursorType::IBeam),
                (250, CursorType::Hand),
            ],
        };
        track.save(&path).unwrap();
        let loaded = CursorTrack::load(&path);
        assert_eq!(loaded.samples.len(), track.samples.len());
        for (a, b) in track.samples.iter().zip(loaded.samples.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = Path::new("C:/nonexistent/path/cursor.json");
        let track = CursorTrack::load(path);
        assert!(track.samples.is_empty());
    }

    #[test]
    fn resize_ns_serializes_correctly() {
        let t = CursorType::ResizeNs;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#""resize_ns""#);
    }

    #[test]
    fn all_variants_serde_roundtrip() {
        let variants = [
            CursorType::Arrow,
            CursorType::IBeam,
            CursorType::Hand,
            CursorType::ResizeNs,
            CursorType::ResizeEw,
            CursorType::ResizeNwse,
            CursorType::ResizeNesw,
            CursorType::Move,
            CursorType::Busy,
        ];
        for v in &variants {
            let s = serde_json::to_string(v).unwrap();
            let back: CursorType = serde_json::from_str(&s).unwrap();
            assert_eq!(*v, back);
        }
    }

    #[test]
    fn expected_serde_names() {
        let pairs = [
            (CursorType::Arrow, "arrow"),
            (CursorType::IBeam, "ibeam"),
            (CursorType::Hand, "hand"),
            (CursorType::ResizeNs, "resize_ns"),
            (CursorType::ResizeEw, "resize_ew"),
            (CursorType::ResizeNwse, "resize_nwse"),
            (CursorType::ResizeNesw, "resize_nesw"),
            (CursorType::Move, "move"),
            (CursorType::Busy, "busy"),
        ];
        for (v, expected) in &pairs {
            let json = serde_json::to_string(v).unwrap();
            assert_eq!(json, format!("\"{}\"", expected), "variant {:?}", v);
        }
    }
}
