use serde::{Deserialize, Serialize};

pub const TEXT_SIZE_FRACS: [f32; 5] = [0.030, 0.042, 0.058, 0.082, 0.115];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    #[default]
    Title,
    LowerThird,
    Stat,
    Callout,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    MidLeft,
    MidCenter,
    MidRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    Xs,
    S,
    #[default]
    M,
    L,
    Xl,
}

impl TextSize {
    pub fn frac(self) -> f32 {
        TEXT_SIZE_FRACS[self as usize]
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextAnim {
    #[default]
    Fade,
    Slide,
    Pop,
    Typewriter,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextItem {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    #[serde(default)]
    pub kind: TextKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default = "default_text_style")]
    pub style: String,
    #[serde(default)]
    pub pos: TextAnchor,
    #[serde(default)]
    pub offset: [f32; 2],
    #[serde(default)]
    pub size: TextSize,
    #[serde(default)]
    pub anim_in: TextAnim,
    #[serde(default)]
    pub anim_out: TextAnim,
    #[serde(default = "default_anim_ms")]
    pub in_ms: u32,
    #[serde(default = "default_anim_ms")]
    pub out_ms: u32,
    #[serde(default = "default_text_easing")]
    pub easing: String,
}

pub fn default_text_style() -> String {
    "clean".into()
}
pub fn default_anim_ms() -> u32 {
    420
}
pub fn default_text_easing() -> String {
    "smooth".into()
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
