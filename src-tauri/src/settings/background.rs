use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind {
    Mesh,
    Solid,
    Gradient,
    Image,
    Video,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub kind: BackgroundKind,
    pub solid: [u8; 3],
    pub gradient_from: [u8; 3],
    pub gradient_to: [u8; 3],
    pub gradient_angle_deg: f32,
    pub blur: f32,
    #[serde(default)]
    pub mesh: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_mid: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
    #[serde(default)]
    pub dim: f32,
}
impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            kind: BackgroundKind::Mesh,
            solid: [24, 24, 30],
            gradient_from: [36, 41, 56],
            gradient_to: [88, 64, 120],
            gradient_angle_deg: 135.0,
            blur: 0.0,
            mesh: String::new(),
            gradient_mid: None,
            asset: None,
            dim: 0.0,
        }
    }
}

impl BackgroundSettings {
    pub fn dim_clamped(&self) -> f32 {
        self.dim.clamp(0.0, 0.8)
    }
}

#[cfg(test)]
#[path = "background_tests.rs"]
mod tests;
