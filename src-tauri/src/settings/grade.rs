use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GradePreset {
    #[default]
    None,
    Cinematic,
    Noir,
    Vintage,
    Frost,
    Golden,
    Midnight,
    Vivid,
    Dreamy,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct GradeSettings {
    pub preset: GradePreset,
    pub exposure: f32,
    pub contrast: f32,
    pub vignette: f32,
}

impl Default for GradeSettings {
    fn default() -> Self {
        Self {
            preset: GradePreset::None,
            exposure: 0.0,
            contrast: 1.0,
            vignette: 0.0,
        }
    }
}

impl GradeSettings {
    pub fn is_identity(&self) -> bool {
        self.preset == GradePreset::None
            && self.exposure == 0.0
            && self.contrast == 1.0
            && self.vignette == 0.0
    }
}

#[cfg(test)]
#[path = "grade_tests.rs"]
mod tests;
