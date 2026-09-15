use serde::{Deserialize, Serialize};

pub const DEFAULT_CRF: u8 = 24;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Resolution {
    #[serde(rename = "p720")]
    P720,
    #[serde(rename = "p1080")]
    P1080,
    #[serde(rename = "p1440")]
    P1440,
    #[serde(rename = "p2160")]
    P2160,
    #[default]
    #[serde(rename = "source")]
    Source,
}

impl Resolution {
    fn short_edge_px(self) -> Option<u32> {
        match self {
            Resolution::P720 => Some(720),
            Resolution::P1080 => Some(1080),
            Resolution::P1440 => Some(1440),
            Resolution::P2160 => Some(2160),
            Resolution::Source => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Fps {
    #[serde(rename = "f30")]
    F30,
    #[default]
    #[serde(rename = "f60")]
    F60,
    #[serde(rename = "source")]
    Source,
}

impl Fps {
    pub fn resolve_hz(self, capture_fps: u32) -> u64 {
        match self {
            Fps::F30 => 30,
            Fps::F60 => 60,
            Fps::Source => {
                if capture_fps == 0 {
                    60
                } else {
                    capture_fps as u64
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    #[serde(rename = "mp4")]
    Mp4,
    #[serde(rename = "webm")]
    WebM,
    #[serde(rename = "gif")]
    Gif,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::WebM => "webm",
            Format::Gif => "gif",
        }
    }

    pub fn supports_audio(self) -> bool {
        !matches!(self, Format::Gif)
    }

    pub fn audio_codec(self) -> &'static str {
        match self {
            Format::Mp4 => "aac",
            Format::WebM => "libopus",
            Format::Gif => "",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ExportSettings {
    pub resolution: Resolution,
    pub fps: Fps,
    pub quality_crf: u8,
    pub format: Format,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            resolution: Resolution::default(),
            fps: Fps::default(),
            quality_crf: DEFAULT_CRF,
            format: Format::default(),
        }
    }
}

impl crate::export::types::Layout {
    pub(crate) fn rescale_to_resolution(&mut self, resolution: Resolution) {
        if let Some(short) = resolution.short_edge_px() {
            let (w, h) = (self.out_w.max(1), self.out_h.max(1));
            if w <= h {
                self.out_w = short & !1;
                self.out_h = (((short as u64) * (h as u64) / (w as u64)) as u32) & !1;
            } else {
                self.out_h = short & !1;
                self.out_w = (((short as u64) * (w as u64) / (h as u64)) as u32) & !1;
            }
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
