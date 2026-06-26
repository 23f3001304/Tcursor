use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Button { Left, Right, Middle }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind { Move, Down, Up }

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
pub struct ScreenInfo { pub w: u32, pub h: u32, pub origin_x: i32, pub origin_y: i32 }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventLog {
    pub started_unix_ms: u64,
    pub screen: ScreenInfo,
    #[serde(default)]
    pub events: Vec<MouseEvent>,
}

impl EventLog {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
    pub fn load(path: &Path) -> io::Result<EventLog> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trips_through_json() {
        let log = EventLog {
            started_unix_ms: 1000,
            screen: ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 },
            events: vec![
                MouseEvent { t: 0, kind: EventKind::Move, x: 5, y: 6, button: None },
                MouseEvent { t: 30, kind: EventKind::Down, x: 5, y: 6, button: Some(Button::Left) },
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
            t: 0, kind: EventKind::Move, x: 1, y: 2, button: None,
        }).unwrap();
        assert!(!json.contains("button"));
    }
}
