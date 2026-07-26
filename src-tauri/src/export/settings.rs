// User-chosen export output settings: resolution (size), fps, quality (CRF), and container
// format. The single place `ExportSettings` maps onto pixel dimensions - `Layout::resolve`
// (types.rs) calls `rescale_to_resolution` below right after `apply_aspect`, so `Aspect` gives
// the RATIO and `Resolution` gives the SIZE, agreeing for every combination.
use serde::{Deserialize, Serialize};

/// Today's export quality (`FfmpegFrameSink`'s old hardcoded "medium" CRF) - the default so an
/// unconfigured export is unchanged.
pub const DEFAULT_CRF: u8 = 24;

/// Output frame SIZE, independent of `Aspect` (the RATIO). Fixed presets name the SHORT edge -
/// the smaller of width/height - in pixels: e.g. `P1080` on a landscape ratio (16:9, 4:3, 1:1) is
/// height=1080 (matches the familiar "1080p"); on a portrait ratio (9:16) the short edge is the
/// WIDTH, so `P1080` there is 1080x1920. `Source` is a no-op: dims stay whatever `apply_aspect`
/// (or, for `Aspect::Source`, `adapt_to_source`) already resolved - today's exact behavior.
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
    /// The short edge in px for a fixed preset, or `None` for `Source` (no override).
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

/// Output frame rate. `Source` reproduces today's exact fallback formula (the capture/display
/// refresh rate, capped at 60) via `resolve_hz`; the default `F60` is a fixed 60 regardless of
/// the display, which is what that fallback formula already produces on effectively every real
/// machine (`primary_refresh_hz` rarely returns under 60) - so the default output is unchanged.
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
    /// Resolve to an actual encode rate (frames per second). `capture_fps` is the SAME display-
    /// refresh-derived value `exporter::export` already computes for `FrameRenderer::new`'s
    /// capture-rate fallback - `Source` reuses it instead of re-querying the display, applying
    /// the exact `if 0 {60} else {..}` guard the old unconditional formula used.
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

/// Export container/codec choice.
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
    /// Output file extension - also the `tmp_export.<ext>` intermediate's extension.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::WebM => "webm",
            Format::Gif => "gif",
        }
    }

    /// Whether this container can carry an audio track at all. `Gif` cannot, so
    /// `audio_mux::mux` always takes its "no audio" (rename-only) path for it, regardless of
    /// whether mic/system audio was recorded.
    pub fn supports_audio(self) -> bool {
        !matches!(self, Format::Gif)
    }

    /// ffmpeg audio encoder name used by the final mux step (empty when `!supports_audio`).
    pub fn audio_codec(self) -> &'static str {
        match self {
            Format::Mp4 => "aac",
            Format::WebM => "libopus",
            Format::Gif => "",
        }
    }
}

/// User-chosen export settings, collected by `ExportDialog` and threaded through
/// `export_project` -> `run_export` -> `exporter::export` into `Layout` (dims), the encode loop
/// (fps), and `FfmpegFrameSink`/`audio_mux::mux` (quality + container). `Default` reproduces
/// today's export exactly: `Source` resolution (aspect-adapt dims unchanged), 60fps, CRF 24,
/// MP4/H.264. `#[serde(default)]` means a partially-specified settings object (or, defensively,
/// an empty one) still fills in every missing field from these same defaults.
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
    /// Rescale the already aspect-resolved `out_w`/`out_h` to `resolution`'s SHORT edge,
    /// preserving the exact ratio `apply_aspect` produced (see `Resolution`'s own doc for the
    /// short-edge convention). Called from `Layout::resolve` right after `apply_aspect`, so
    /// export/preview callers never invoke this directly. A no-op for `Resolution::Source`.
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
