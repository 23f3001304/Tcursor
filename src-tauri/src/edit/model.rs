use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Trim { pub in_ms: u32, pub out_ms: u32 }
impl Default for Trim { fn default() -> Self { Self { in_ms: 0, out_ms: 0 } } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cut { pub start_ms: u32, pub end_ms: u32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget { Cursor, Fixed { x: f32, y: f32 } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String, pub start_ms: u32, pub end_ms: u32,
    pub target: ZoomTarget, pub scale: f32, pub easing: String,
    #[serde(default = "default_zoom_in_ms")] pub zoom_in_ms: u32,
    #[serde(default = "default_zoom_out_ms")] pub zoom_out_ms: u32,
    /// Priority when this zoom overlaps another in time - higher wins. Also determines
    /// which timeline row it renders on. Auto-assigned on creation (see `edit::ops::api::auto_layer`),
    /// user-overridable via `UpdateZoom`.
    #[serde(default)] pub layer: u32,
}

fn default_zoom_in_ms() -> u32 { 350 }
fn default_zoom_out_ms() -> u32 { 450 }
/// Matches `fx_state::FADE_MS` (the spotlight fade baseline).
fn default_fade_ms() -> u32 { 250 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed { pub id: String, pub start_ms: u32, pub end_ms: u32, pub factor: f32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg {
    pub id: String, pub start_ms: u32, pub end_ms: u32, pub layout: String,
    /// Cross-fade duration (ms) INTO this layout - the editable transition feel.
    #[serde(default = "default_layout_transition_ms")] pub transition_ms: u32,
    /// Easing wire-name for the fade ("linear" | "smooth" | "spring").
    #[serde(default = "default_layout_easing")] pub easing: String,
}
fn default_layout_transition_ms() -> u32 { 350 }
fn default_layout_easing() -> String { "smooth".into() }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraMove {
    pub id: String, pub t_ms: u32, pub x: f32, pub y: f32, pub size: f32,
    #[serde(default = "default_cam_easing")] pub easing: String,
}
fn default_cam_easing() -> String { "smooth".into() }

/// An editable effect region on the timeline. v1 covers Spotlight; the kind grows over phases.
/// Params default from settings for now (per-region overrides are a later addition).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion {
    pub id: String, pub kind: EffectKind, pub start_ms: u32, pub end_ms: u32,
    #[serde(default = "default_fade_ms")] pub fade_in_ms: u32,
    #[serde(default = "default_fade_ms")] pub fade_out_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub mode: Option<crate::settings::model::SpotlightMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub dim: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub feather: Option<f32>,
    /// Priority when this region overlaps another Spotlight region - higher wins.
    #[serde(default)] pub layer: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    pub version: u32,
    pub trim: Trim,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    #[serde(default)]
    pub effects: Vec<EffectRegion>,
    #[serde(default)]
    pub camera_moves: Vec<CameraMove>,
    pub settings: crate::settings::model::Settings,
}
impl Default for EditDoc {
    fn default() -> Self {
        Self { version: 1, trim: Trim::default(), cuts: vec![], zooms: vec![], speed: vec![], layout: vec![], effects: vec![], camera_moves: vec![], settings: crate::settings::model::Settings::default() }
    }
}

impl EditDoc {
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
    }
    pub fn load(path: &std::path::Path) -> Option<EditDoc> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
