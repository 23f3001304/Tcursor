use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MotionSettings {
    pub preset: String,
    pub easing: String,
    pub easing_out: String,
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            preset: "soft".into(),
            easing: "smooth".into(),
            easing_out: "smooth".into(),
        }
    }
}

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;
