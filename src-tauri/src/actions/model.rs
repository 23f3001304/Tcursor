use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutId {
    Screen,
    Camera,
    Presenter,
    ScreenOnly,
    CameraOnly,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    SetLayout(LayoutId),
    ZoomHoldStart,
    ZoomHoldEnd,
    SpotlightHoldStart,
    SpotlightHoldEnd,
    VideoFxHoldStart,
    VideoFxHoldEnd,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEvent {
    pub t: u32,
    pub kind: ActionKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ActionLog {
    #[serde(default)]
    pub actions: Vec<ActionEvent>,
}

impl ActionLog {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
    pub fn load(path: &Path) -> io::Result<ActionLog> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trips_actions_json() {
        let log = ActionLog {
            actions: vec![
                ActionEvent {
                    t: 100,
                    kind: ActionKind::SetLayout(LayoutId::Camera),
                },
                ActionEvent {
                    t: 250,
                    kind: ActionKind::ZoomHoldStart,
                },
                ActionEvent {
                    t: 900,
                    kind: ActionKind::ZoomHoldEnd,
                },
            ],
        };
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"set_layout\":\"camera\""));
        assert!(json.contains("\"zoom_hold_start\""));
        let back: ActionLog = serde_json::from_str(&json).unwrap();
        assert_eq!(back.actions.len(), 3);
        assert_eq!(
            back.actions[0].kind,
            ActionKind::SetLayout(LayoutId::Camera)
        );
        assert_eq!(back.actions[2].kind, ActionKind::ZoomHoldEnd);
    }
}
