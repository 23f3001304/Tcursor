use serde::{Deserialize, Serialize};

/// Which of `BackgroundSettings`' fields the renderer uses to build the background buffer
/// (`export::scene::background::build`). `Mesh` (the default) covers both the legacy bundled
/// `bg.jpg` (when `mesh` is empty) and the procedural wallpaper library (`settings::wallpapers`),
/// so every recording/config saved before the library existed keeps its exact look.
/// `Solid`/`Gradient` are real user-chosen colors, rendered directly (no ffmpeg decode).
/// `Image`/`Video` render a user-supplied file copied into the project (`BackgroundSettings.asset`,
/// see `settings::bg_asset`); a GIF is a `Video`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind { Mesh, Solid, Gradient, Image, Video }

/// User-facing background settings, part of `Settings` (persisted in both `config.json` and
/// per-project `edit.json`). See `export::scene::background::build` for how these become pixels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub kind: BackgroundKind,
    pub solid: [u8; 3],
    pub gradient_from: [u8; 3],
    pub gradient_to: [u8; 3],
    pub gradient_angle_deg: f32,
    /// 0..1 softness applied once to the STATIC background buffer (cheap: that buffer is built
    /// once per export/preview, never per frame). 0 = off (today's behavior).
    pub blur: f32,
    /// Which procedural wallpaper (`settings::wallpapers::MESH_WALLPAPERS` id) `Mesh` renders.
    /// EMPTY = the legacy bundled `bg.jpg`, which is what every pre-library doc deserializes to,
    /// so those keep rendering byte-identically. An unknown id falls back to the same image.
    #[serde(default)] pub mesh: String,
    /// Optional middle stop for `Gradient`. `None` (the default, and what every pre-library doc
    /// loads) leaves the two-stop ramp exactly as it was.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub gradient_mid: Option<[u8; 3]>,
    /// The user's imported background file, RELATIVE to the project folder (`background/<file>`),
    /// used by `Image`/`Video`. Never an absolute path - projects have to stay portable, so
    /// `bg_asset::asset_path` refuses anything absolute or containing `..`. Kept on disk (and in
    /// this field) when the user switches back to a wallpaper, so re-selecting it needs no
    /// re-import. Absent from the JSON entirely when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub asset: Option<String>,
    /// 0..0.8 black overlay drawn over whichever background actually has pixels (wallpaper, image
    /// or video). Default 0. Read through `dim_clamped`, never raw.
    #[serde(default)] pub dim: f32,
}
impl Default for BackgroundSettings {
    fn default() -> Self {
        Self { kind: BackgroundKind::Mesh, solid: [24, 24, 30],
            gradient_from: [36, 41, 56], gradient_to: [88, 64, 120], gradient_angle_deg: 135.0, blur: 0.0,
            mesh: String::new(), gradient_mid: None, asset: None, dim: 0.0 }
    }
}

impl BackgroundSettings {
    /// `dim` inside its published range. The panel's slider is already 0..80%, so this only ever
    /// matters for a hand-edited `edit.json`; 0.8 is the floor on legibility (a fully black
    /// background is what `Solid` is for).
    pub fn dim_clamped(&self) -> f32 { self.dim.clamp(0.0, 0.8) }
}

#[cfg(test)]
#[path = "background_tests.rs"]
mod tests;
