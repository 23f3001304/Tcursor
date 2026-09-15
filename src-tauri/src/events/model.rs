use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    Move,
    Down,
    Up,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct MouseEvent {
    pub t: u32,
    pub kind: EventKind,
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<Button>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenInfo {
    pub w: u32,
    pub h: u32,
    pub origin_x: i32,
    pub origin_y: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventLog {
    pub started_unix_ms: u64,
    pub screen: ScreenInfo,
    #[serde(default)]
    pub events: Vec<MouseEvent>,
}

impl EventLog {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        serde_json::to_writer(&mut encoder, self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        encoder.finish()?;
        Ok(())
    }
    pub fn load(path: &Path) -> io::Result<EventLog> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path)?;
        let mut header = [0u8; 2];
        let is_gzip = if file.read_exact(&mut header).is_ok() {
            header == [0x1f, 0x8b]
        } else {
            false
        };
        file.seek(SeekFrom::Start(0))?;
        let reader = std::io::BufReader::new(file);
        if is_gzip {
            let decoder = GzDecoder::new(reader);
            serde_json::from_reader(decoder).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        } else {
            serde_json::from_reader(reader).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trips_through_json() {
        let log = EventLog {
            started_unix_ms: 1000,
            screen: ScreenInfo {
                w: 1920,
                h: 1080,
                origin_x: 0,
                origin_y: 0,
            },
            events: vec![
                MouseEvent {
                    t: 0,
                    kind: EventKind::Move,
                    x: 5,
                    y: 6,
                    button: None,
                },
                MouseEvent {
                    t: 30,
                    kind: EventKind::Down,
                    x: 5,
                    y: 6,
                    button: Some(Button::Left),
                },
            ],
        };
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"kind\":\"move\""));
        assert!(json.contains("\"button\":\"left\""));
        let back: EventLog = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 2);
        assert_eq!(back.events[1].button, Some(Button::Left));
    }
    #[test]
    fn omits_button_for_moves() {
        let json = serde_json::to_string(&MouseEvent {
            t: 0,
            kind: EventKind::Move,
            x: 1,
            y: 2,
            button: None,
        })
        .unwrap();
        assert!(!json.contains("button"));
    }
    #[test]
    fn loads_both_compressed_and_uncompressed_event_logs() {
        let log = EventLog {
            started_unix_ms: 1000,
            screen: ScreenInfo {
                w: 1920,
                h: 1080,
                origin_x: 0,
                origin_y: 0,
            },
            events: vec![MouseEvent {
                t: 0,
                kind: EventKind::Move,
                x: 5,
                y: 6,
                button: None,
            }],
        };
        let tmp_dir = std::env::temp_dir();
        let gz_path = tmp_dir.join("test_events_gz.json");
        let plain_path = tmp_dir.join("test_events_plain.json");
        let _ = std::fs::remove_file(&gz_path);
        let _ = std::fs::remove_file(&plain_path);

        log.save(&gz_path).unwrap();
        let json = serde_json::to_vec(&log).unwrap();
        std::fs::write(&plain_path, json).unwrap();

        let log_gz = EventLog::load(&gz_path).unwrap();
        let log_plain = EventLog::load(&plain_path).unwrap();

        assert_eq!(log_gz.started_unix_ms, 1000);
        assert_eq!(log_plain.started_unix_ms, 1000);
        assert_eq!(log_gz.events.len(), 1);
        assert_eq!(log_plain.events.len(), 1);

        let _ = std::fs::remove_file(&gz_path);
        let _ = std::fs::remove_file(&plain_path);
    }
}
