use crate::edit::model::default_fade_ms;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    Spotlight,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion {
    pub id: String,
    pub kind: EffectKind,
    pub start_ms: u32,
    pub end_ms: u32,
    #[serde(default = "default_fade_ms")]
    pub fade_in_ms: u32,
    #[serde(default = "default_fade_ms")]
    pub fade_out_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::settings::model::SpotlightMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feather: Option<f32>,
    #[serde(default)]
    pub layer: u32,
}
