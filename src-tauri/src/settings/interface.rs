use crate::settings::model::default_true;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct InterfaceSettings {
    pub theme: ThemeMode,
    pub accent: [u8; 3],
    pub animated_brand: bool,
    #[serde(default = "default_true")]
    pub interface_effects: bool,
    #[serde(default)]
    pub ai_choreography: bool,
}
impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Light,
            accent: [239, 68, 68],
            animated_brand: true,
            interface_effects: true,
            ai_choreography: false,
        }
    }
}
