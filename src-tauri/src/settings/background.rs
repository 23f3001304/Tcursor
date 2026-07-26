use serde::{Deserialize, Serialize};

/// Which of `BackgroundSettings`' fields the renderer uses to build the static background
/// buffer (`export::scene::background::build`). `Mesh` (the default) reproduces today's fixed
/// `bg.jpg` exactly, so every recording/config saved before this field existed keeps its exact
/// look. `Solid`/`Gradient` are real user-chosen colors, rendered directly (no ffmpeg decode).
/// Custom image/video backgrounds are a future subsystem (need an upload + asset pipeline) and
/// are intentionally NOT modeled here - the editor flags them as "coming soon" instead of
/// faking a variant with no real content behind it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind { Mesh, Solid, Gradient }

/// User-facing background settings, part of `Settings` (persisted in both `config.json` and
/// per-project `edit.json`). See `export::scene::background::build` for how these become pixels.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub kind: BackgroundKind,
    pub solid: [u8; 3],
    pub gradient_from: [u8; 3],
    pub gradient_to: [u8; 3],
    pub gradient_angle_deg: f32,
    /// 0..1 softness applied once to the static background buffer (cheap: `bg` is built once
    /// per export/preview, never per frame). 0 = off (today's behavior).
    pub blur: f32,
}
impl Default for BackgroundSettings {
    fn default() -> Self {
        Self { kind: BackgroundKind::Mesh, solid: [24, 24, 30],
            gradient_from: [36, 41, 56], gradient_to: [88, 64, 120], gradient_angle_deg: 135.0, blur: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_mesh_and_round_trips() {
        let s = BackgroundSettings::default();
        assert_eq!(s.kind, BackgroundKind::Mesh);
        assert_eq!(s.blur, 0.0);
        let json = serde_json::to_string(&s).unwrap();
        let back: BackgroundSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
    #[test]
    fn partial_json_fills_defaults() {
        let back: BackgroundSettings = serde_json::from_str("{\"kind\":\"solid\",\"solid\":[1,2,3]}").unwrap();
        assert_eq!(back.kind, BackgroundKind::Solid);
        assert_eq!(back.solid, [1, 2, 3]);
        assert_eq!(back.gradient_angle_deg, 135.0); // untouched fields keep defaults
    }
}
