use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Trim { pub in_ms: u32, pub out_ms: u32 }
impl Default for Trim { fn default() -> Self { Self { in_ms: 0, out_ms: 0 } } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cut { pub start_ms: u32, pub end_ms: u32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget { Cursor, Fixed { x: f32, y: f32 } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String, pub start_ms: u32, pub end_ms: u32,
    pub target: ZoomTarget, pub scale: f32, pub easing: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed { pub id: String, pub start_ms: u32, pub end_ms: u32, pub factor: f32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg { pub id: String, pub start_ms: u32, pub end_ms: u32, pub layout: String }

/// An editable effect region on the timeline. v1 covers Spotlight; the kind grows over phases.
/// Params default from settings for now (per-region overrides are a later addition).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion { pub id: String, pub kind: EffectKind, pub start_ms: u32, pub end_ms: u32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    pub version: u32,
    pub trim: Trim,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    #[serde(default)]
    pub effects: Vec<EffectRegion>,
    pub settings: crate::settings::model::Settings,
}
impl Default for EditDoc {
    fn default() -> Self {
        Self { version: 1, trim: Trim::default(), cuts: vec![], zooms: vec![], speed: vec![], layout: vec![], effects: vec![], settings: crate::settings::model::Settings::default() }
    }
}

impl EditDoc {
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
    }
    pub fn load(path: &std::path::Path) -> Option<EditDoc> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn sample_doc() -> EditDoc {
        EditDoc {
            version: 1,
            trim: Trim { in_ms: 100, out_ms: 5000 },
            cuts: vec![Cut { start_ms: 500, end_ms: 1000 }],
            zooms: vec![Zoom { id: "z1".into(), start_ms: 200, end_ms: 800, target: ZoomTarget::Cursor, scale: 2.2, easing: "ease".into() }],
            speed: vec![Speed { id: "s1".into(), start_ms: 1000, end_ms: 2000, factor: 2.0 }],
            layout: vec![LayoutSeg { id: "l1".into(), start_ms: 0, end_ms: 5000, layout: "screen".into() }],
            effects: vec![],
            settings: crate::settings::model::Settings::default(),
        }
    }

    #[test]
    fn round_trip_save_load() {
        let doc = sample_doc();
        let p = tmp_path("edit_model_round_trip.json");
        doc.save(&p).unwrap();
        let loaded = EditDoc::load(&p).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.trim.in_ms, 100);
        assert_eq!(loaded.cuts.len(), 1);
        assert_eq!(loaded.zooms.len(), 1);
        assert_eq!(loaded.zooms[0].id, "z1");
        assert_eq!(loaded.speed.len(), 1);
        assert_eq!(loaded.layout.len(), 1);
        assert_eq!(loaded, doc);
    }

    #[test]
    fn load_missing_path_is_none() {
        let p = tmp_path("edit_model_no_such_file_xyz.json");
        let _ = std::fs::remove_file(&p);
        assert!(EditDoc::load(&p).is_none());
    }

    #[test]
    fn partial_json_fills_defaults() {
        let json = r#"{"zooms":[{"id":"z1","start_ms":0,"end_ms":100,"target":"cursor","scale":2.0,"easing":"linear"}]}"#;
        let doc: EditDoc = serde_json::from_str(json).unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.cuts.len(), 0);
        assert_eq!(doc.speed.len(), 0);
        assert_eq!(doc.layout.len(), 0);
        assert_eq!(doc.trim, Trim::default());
        assert_eq!(doc.zooms.len(), 1);
    }

    #[test]
    fn zoom_target_fixed_serializes_with_xy() {
        let t = ZoomTarget::Fixed { x: 0.5, y: 0.75 };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"x\""), "x missing: {}", json);
        assert!(json.contains("\"y\""), "y missing: {}", json);
        assert!(json.contains("fixed"), "variant missing: {}", json);
        let back: ZoomTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
