use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionPos {
    #[default]
    Bottom,
    Top,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionSize {
    S,
    #[default]
    M,
    L,
}
impl CaptionSize {
    pub fn height_frac(self) -> f32 {
        match self {
            CaptionSize::S => 0.030,
            CaptionSize::M => 0.038,
            CaptionSize::L => 0.048,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionAnim {
    None,
    #[default]
    Fade,
    Rise,
    Pop,
    Words,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CaptionStyle {
    pub enabled: bool,
    pub position: CaptionPos,
    pub size: CaptionSize,
    pub pill: bool,
    pub highlight: bool,
    pub model: String,
    pub language: String,
    pub font_pct: f32,
    pub text_color: [u8; 3],
    pub highlight_color: Option<[u8; 3]>,
    pub pill_color: [u8; 3],
    pub pill_alpha: u8,
    pub animation: CaptionAnim,
    pub animation_ms: u32,
}
impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            position: CaptionPos::Bottom,
            size: CaptionSize::M,
            pill: true,
            highlight: true,
            model: "base.en".into(),
            language: "en".into(),
            font_pct: 0.0,
            text_color: [255, 255, 255],
            highlight_color: None,
            pill_color: [0, 0, 0],
            pill_alpha: 62,
            animation: CaptionAnim::Fade,
            animation_ms: 120,
        }
    }
}
impl CaptionStyle {
    pub fn height_frac(&self) -> f32 {
        if self.font_pct > 0.0 {
            (self.font_pct / 100.0).clamp(0.015, 0.08)
        } else {
            self.size.height_frac()
        }
    }
}

#[cfg(test)]
#[path = "captions_tests.rs"]
mod tests;
