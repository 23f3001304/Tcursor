use serde::{Deserialize, Serialize};
use crate::export::types::ZoomConfig;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ZoomSettings { pub enabled: bool, pub target_scale: f32, pub hold_ms: u32, pub smoothness: f32, pub clicks: u32 }
impl Default for ZoomSettings {
    fn default() -> Self { Self { enabled: true, target_scale: 2.2, hold_ms: 2200, smoothness: 0.10, clicks: 1 } }
}
impl ZoomSettings {
    /// Full ZoomConfig from the user-facing subset; other fields keep tuned defaults.
    pub fn to_zoom_config(&self) -> ZoomConfig {
        ZoomConfig {
            target_scale: self.target_scale,
            idle_release_ms: self.hold_ms,
            follow_damping: self.smoothness,
            clicks_to_trigger: self.clicks.max(1),
            ..ZoomConfig::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClickFxStyle { None, Ripple, Pulse }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ClickFxSettings {
    pub enabled: bool, pub style: ClickFxStyle, pub color: [u8; 3],
    pub intensity: f32, pub captions: bool, pub spotlight: bool,
}
impl Default for ClickFxSettings {
    fn default() -> Self { Self { enabled: true, style: ClickFxStyle::Ripple, color: [255, 255, 255], intensity: 0.8, captions: false, spotlight: false } }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct HotkeySettings {
    pub zoom_hold: String,
    pub layout_screen: String, pub layout_camera: String, pub layout_presenter: String,
    pub layout_screen_only: String, pub layout_camera_only: String,
}
impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            zoom_hold: "Ctrl+Alt+Z".into(),
            layout_screen: "Ctrl+Alt+1".into(), layout_camera: "Ctrl+Alt+2".into(),
            layout_presenter: "Ctrl+Alt+3".into(), layout_screen_only: "Ctrl+Alt+4".into(),
            layout_camera_only: "Ctrl+Alt+5".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    pub zoom: ZoomSettings,
    pub clickfx: ClickFxSettings,
    pub hotkeys: HotkeySettings,
    /// Manual mic-vs-video sync nudge in ms (negative pulls the mic earlier, to
    /// cancel the mic's device input latency). 0 = off. Applied to the mic at mux.
    pub audio_offset_ms: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_tuned_zoom_and_round_trip() {
        let s = Settings::default();
        assert!(s.zoom.enabled);
        assert_eq!(s.zoom.target_scale, 2.2);
        assert_eq!(s.zoom.hold_ms, 2200);
        let cfg = s.zoom.to_zoom_config();
        assert_eq!(cfg.target_scale, 2.2);
        assert_eq!(cfg.idle_release_ms, 2200);
        assert_eq!(cfg.follow_damping, 0.10);
        assert_eq!(cfg.zoom_in_ms, 350); // untouched ZoomConfig default
        assert_eq!(s.zoom.clicks, 1);
        assert_eq!(cfg.clicks_to_trigger, 1);
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
    #[test]
    fn partial_json_fills_defaults() {
        let back: Settings = serde_json::from_str("{\"zoom\":{\"enabled\":false}}").unwrap_or_default();
        // missing fields fall back to defaults
        assert!(!back.zoom.enabled);
        assert_eq!(back.zoom.target_scale, 2.2);
        assert_eq!(back.clickfx.enabled, ClickFxSettings::default().enabled);
    }
}
