use serde::{Deserialize, Serialize};
use crate::export::types::ZoomConfig;
use crate::settings::appearance::AppearanceSettings;
use crate::settings::background::BackgroundSettings;
use crate::settings::cursor::CursorSettings;

/// What the webcam PiP does while a zoom is active. `Shrink` is today's behavior (the panel
/// scales toward `to` as the zoom deepens), `Hide` fades it out on the same curve, `Stay`
/// leaves it untouched. Resolved per-zoom (`Zoom.cam_action`), falling back to the global
/// `ZoomSettings::resolved_cam_action`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CamZoomAction { Shrink { to: f32 }, Hide, Stay }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ZoomSettings { pub enabled: bool, pub target_scale: f32, pub hold_ms: u32, pub smoothness: f32, pub clicks: u32, pub camera_shrink: bool, pub camera_shrink_min: f32, pub smart_hold: bool, pub smart_follow: bool,
    /// Global default webcam-on-zoom action. `None` = derive it from the legacy
    /// `camera_shrink`/`camera_shrink_min` pair (see `resolved_cam_action`).
    pub cam_zoom_default: Option<CamZoomAction>,
    /// Opt-in critically-damped smoothing pass on the auto-zoom CAMERA path (`ZoomConfig::smoothing_ms`,
    /// see `export/camera/smoothing.rs`) - NOT the cursor low-pass (`CursorSettings::smoothness`).
    /// 0 = off, bit-identical to today. `#[serde(default)]` so configs saved before this field
    /// existed load with 0, matching `smoothing_off_is_bit_identical`.
    #[serde(default)]
    pub camera_smoothing_ms: u32 }
impl Default for ZoomSettings {
    fn default() -> Self { Self { enabled: true, target_scale: 2.2, hold_ms: 2200, smoothness: 0.10, clicks: 1, camera_shrink: true, camera_shrink_min: 0.62, smart_hold: true, smart_follow: false, cam_zoom_default: None, camera_smoothing_ms: 0 } }
}
impl ZoomSettings {
    /// The global default action. Derived from the legacy `camera_shrink`/`camera_shrink_min`
    /// pair when `cam_zoom_default` is unset, so settings written before this field existed
    /// resolve to EXACTLY today's behavior (shrink to 0.62, or `Stay` when the toggle is off).
    pub fn resolved_cam_action(&self) -> CamZoomAction {
        self.cam_zoom_default.unwrap_or(if self.camera_shrink {
            CamZoomAction::Shrink { to: self.camera_shrink_min }
        } else { CamZoomAction::Stay })
    }

    /// Full ZoomConfig from the user-facing subset; other fields keep tuned defaults.
    pub fn to_zoom_config(&self) -> ZoomConfig {
        ZoomConfig {
            target_scale: self.target_scale,
            idle_release_ms: self.hold_ms,
            follow_damping: self.smoothness,
            clicks_to_trigger: self.clicks.max(1),
            smoothing_ms: self.camera_smoothing_ms,
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

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ClickFxSettings {
    pub enabled: bool, pub style: ClickFxStyle, pub color: [u8; 3],
    pub intensity: f32, pub captions: bool, pub spotlight: bool,
    pub spotlight_dim: f32, pub spotlight_radius: f32, pub spotlight_feather: f32,
    pub spotlight_mode: SpotlightMode, pub spotlight_tint: [u8; 3],
    pub video_fx_mode: VideoFxMode,
    #[serde(default = "default_true")] pub spotlight_dim_camera: bool,
}
impl Default for ClickFxSettings {
    fn default() -> Self { Self { enabled: true, style: ClickFxStyle::Ripple, color: [255, 255, 255], intensity: 0.8, captions: false, spotlight: false, spotlight_dim: 0.60, spotlight_radius: 0.13, spotlight_feather: 0.10, spotlight_mode: SpotlightMode::Classic, spotlight_tint: [130, 90, 255], video_fx_mode: VideoFxMode::NebulaWash, spotlight_dim_camera: true } }
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct InterfaceSettings {
    pub theme: ThemeMode,
    pub accent: [u8; 3],
    /// The fake-polish "feel knob" (Task 39) for the living brand mark: whether `TcursorMark`
    /// flows/pulses for its recording/exporting/directing states at all, in the HUD and the
    /// editor's `TopBar`. `prefers-reduced-motion` disables the animation regardless of this flag
    /// (accessibility wins over a feel setting); this flag alone lets a user opt out even without
    /// a system-level reduced-motion preference. Doesn't affect the dynamic Windows icon/taskbar
    /// progress (Task 39B) - those are OS chrome, not an in-page animation.
    pub animated_brand: bool,
    /// The second feel knob, for the editor's own interface micro-interactions (2026-09-14): the
    /// click ripple that blooms under every pointerdown in the editor chrome, and the magnetic pull
    /// the transport's Play button and Trim pills exert on a pointer that comes close. Off unmounts
    /// the ripple overlay entirely (no listeners at all) and turns the magnetic hook into a no-op -
    /// see `src/editor/effects/`. Like `animated_brand`, `prefers-reduced-motion` softens these
    /// regardless of this flag (ripples stop growing, the pull stops); this flag is the opt-out for
    /// a user with no system-level preference. Never touches the EXPORT - these are TCursor's own
    /// chrome, not the recording's click effects (`ClickFxSettings`).
    ///
    /// The explicit field default (belt-and-suspenders alongside the container `#[serde(default)]`
    /// and the manual `impl Default` below, matching `Settings::audio_mic_volume`) is what makes a
    /// config.json written before this field existed load `true` rather than `bool::default()`,
    /// which would silently ship the feature turned off to every existing install.
    #[serde(default = "default_true")]
    pub interface_effects: bool,
}
impl Default for InterfaceSettings {
    fn default() -> Self { Self { theme: ThemeMode::Light, accent: [239, 68, 68], animated_brand: true, interface_effects: true } }
}

/// One saved "look": a name plus a snapshot of ALL FIVE layouts' appearance. Lives in the app
/// config rather than in a recording's `edit.json`, which is what lets the editor's Layouts panel
/// apply the same look to a project recorded months later. `id` is opaque and stable (rename
/// changes `name` only), so a row keeps its identity across a rename.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutPreset {
    pub id: String,
    pub name: String,
    pub appearance: AppearanceSettings,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    pub background: BackgroundSettings,
    /// Volume multiplier applied to the mic track at mux (0 = muted, 1 = unchanged, up to 1.5).
    /// Explicit field default (belt-and-suspenders alongside the manual `impl Default` below,
    /// matching `spotlight_dim_camera`'s pattern) so a config saved without this key loads 1.0,
    /// not `f32::default() == 0.0` (silently muted audio).
    #[serde(default = "default_volume")] pub audio_mic_volume: f32,
    /// Volume multiplier applied to the system-audio track at mux. Same range as `audio_mic_volume`.
    #[serde(default = "default_volume")] pub audio_sys_volume: f32,
    /// Ollama model name for the AI director. Empty = let the backend pick its own default
    /// (`"llama3.2"`), so configs saved before this field existed behave identically.
    pub ai_model: String,
    /// The user's saved layout looks, newest last - the editor's Layouts panel reads and writes
    /// this whole list through `get_settings`/`set_settings`. `#[serde(default)]` (belt and
    /// braces alongside the container's own `#[serde(default)]`) so a config written before
    /// presets existed loads with an empty list instead of failing.
    #[serde(default)]
    pub layout_presets: Vec<LayoutPreset>,
}
fn default_volume() -> f32 { 1.0 }
impl Default for Settings {
    // NOT #[derive(Default)]: audio_mic_volume/audio_sys_volume need 1.0 (unity gain), which
    // bare field-type defaults (f32::default() == 0.0, silently muted audio) would get wrong.
    fn default() -> Self {
        Self { zoom: ZoomSettings::default(), clickfx: ClickFxSettings::default(), hotkeys: HotkeySettings::default(),
            appearance: AppearanceSettings::default(), cursor: CursorSettings::default(), ui: InterfaceSettings::default(),
            audio_offset_ms: 0, background: BackgroundSettings::default(),
            audio_mic_volume: 1.0, audio_sys_volume: 1.0, ai_model: String::new(),
            layout_presets: Vec::new() }
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
