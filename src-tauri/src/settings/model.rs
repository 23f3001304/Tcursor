use serde::{Deserialize, Serialize};
use crate::export::types::ZoomConfig;
use crate::settings::appearance::AppearanceSettings;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ZoomSettings { pub enabled: bool, pub target_scale: f32, pub hold_ms: u32, pub smoothness: f32, pub clicks: u32, pub camera_shrink: bool, pub camera_shrink_min: f32, pub smart_hold: bool, pub smart_follow: bool }
impl Default for ZoomSettings {
    fn default() -> Self { Self { enabled: true, target_scale: 2.2, hold_ms: 2200, smoothness: 0.10, clicks: 1, camera_shrink: true, camera_shrink_min: 0.62, smart_hold: true, smart_follow: false } }
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
pub enum ClickFxStyle { None, Ripple, Pulse, Glow, Shockwave, Particles, Neon }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpotlightMode { Classic, Blur, Halo, Breathing, Nebula, Vignette }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoFxMode { NebulaWash, CinematicDim, ScreenFocus, ColorPop }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ClickFxSettings {
    pub enabled: bool, pub style: ClickFxStyle, pub color: [u8; 3],
    pub intensity: f32, pub captions: bool, pub spotlight: bool,
    pub spotlight_dim: f32, pub spotlight_radius: f32, pub spotlight_feather: f32,
    pub spotlight_mode: SpotlightMode, pub spotlight_tint: [u8; 3],
    pub video_fx_mode: VideoFxMode,
}
impl Default for ClickFxSettings {
    fn default() -> Self { Self { enabled: true, style: ClickFxStyle::Ripple, color: [255, 255, 255], intensity: 0.8, captions: false, spotlight: false, spotlight_dim: 0.60, spotlight_radius: 0.13, spotlight_feather: 0.10, spotlight_mode: SpotlightMode::Classic, spotlight_tint: [130, 90, 255], video_fx_mode: VideoFxMode::NebulaWash } }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct HotkeySettings {
    pub zoom_hold: String,
    pub spotlight_hold: String,
    pub video_fx_hold: String,
    pub layout_screen: String, pub layout_camera: String, pub layout_presenter: String,
    pub layout_screen_only: String, pub layout_camera_only: String,
}
impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            zoom_hold: "Ctrl+Alt+Z".into(),
            spotlight_hold: "Ctrl+Alt+S".into(),
            video_fx_hold: "Ctrl+Alt+V".into(),
            layout_screen: "Ctrl+Alt+1".into(), layout_camera: "Ctrl+Alt+2".into(),
            layout_presenter: "Ctrl+Alt+3".into(), layout_screen_only: "Ctrl+Alt+4".into(),
            layout_camera_only: "Ctrl+Alt+5".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode { Light, Dark, System }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct InterfaceSettings { pub theme: ThemeMode, pub accent: [u8; 3] }
impl Default for InterfaceSettings {
    fn default() -> Self { Self { theme: ThemeMode::Light, accent: [239, 68, 68] } }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle { System, Enhanced, Hidden }
impl CursorStyle {
    /// Whether WGC should bake the OS cursor into the capture. Only `System` does;
    /// `Enhanced`/`Hidden` capture without it (we draw our own, or none).
    pub fn captures_os_cursor(self) -> bool { matches!(self, CursorStyle::System) }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,             // scale of the base cursor size (1.0 = default)
    pub motion_blur: f32,      // 0..1 trail strength (0 = off)
    pub click_bounce: bool,
    pub bounce_intensity: f32, // 0..1 dip depth (0.5 = ~0.18 dip, 1.0 = 0.36 dip)
}
impl Default for CursorSettings {
    fn default() -> Self { Self { style: CursorStyle::System, size: 1.0, motion_blur: 0.35, click_bounce: true, bounce_intensity: 0.5 } }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    pub zoom: ZoomSettings,
    pub clickfx: ClickFxSettings,
    pub hotkeys: HotkeySettings,
    pub appearance: AppearanceSettings,
    pub cursor: CursorSettings,
    pub ui: InterfaceSettings,
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
        assert!(s.zoom.camera_shrink);
        assert_eq!(s.zoom.camera_shrink_min, 0.62);
        assert!(s.zoom.smart_hold);
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
        // old JSON without spotlight_dim/radius/feather loads with defaults
        assert_eq!(back.clickfx.spotlight_dim, 0.60);
        assert_eq!(back.clickfx.spotlight_radius, 0.13);
        assert_eq!(back.clickfx.spotlight_feather, 0.10);
        assert_eq!(back.clickfx.spotlight_mode, SpotlightMode::Classic);
        assert_eq!(back.clickfx.spotlight_tint, [130, 90, 255]);
        // old JSON without camera_shrink/camera_shrink_min loads with defaults
        assert!(back.zoom.camera_shrink);
        assert_eq!(back.zoom.camera_shrink_min, 0.62);
        // old JSON without smart_hold loads with default
        assert!(back.zoom.smart_hold);
        // old JSON without appearance loads the per-mode defaults
        assert_eq!(back.appearance, crate::settings::appearance::AppearanceSettings::default());
        // old JSON without cursor loads the System default (cursor stays baked-in)
        assert_eq!(back.cursor, crate::settings::model::CursorSettings::default());
        assert_eq!(back.cursor.style, CursorStyle::System);
        assert_eq!(CursorSettings::default().bounce_intensity, 0.5);
        assert!(CursorStyle::System.captures_os_cursor());
        assert!(!CursorStyle::Enhanced.captures_os_cursor());
        assert!(!CursorStyle::Hidden.captures_os_cursor());
        // old JSON without ui loads the Light theme + red accent defaults
        assert_eq!(back.ui, crate::settings::model::InterfaceSettings::default());
        assert_eq!(back.ui.theme, ThemeMode::Light);
        assert_eq!(back.ui.accent, [239, 68, 68]);
    }
}
