use serde::{Deserialize, Serialize};

pub const DOC_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct Trim {
    pub in_ms: u32,
    pub out_ms: u32,
}
impl Default for Trim {
    fn default() -> Self {
        Self {
            in_ms: 0,
            out_ms: 0,
        }
    }
}
impl Trim {
    pub fn resolve(&self, total_dur_ms: u32) -> (u32, u32) {
        let out = if self.out_ms == 0 {
            total_dur_ms
        } else {
            self.out_ms.min(total_dur_ms)
        };
        let inp = self.in_ms.min(out);
        (inp, out)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]

pub struct Cut {
    #[serde(default)]
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget {
    Cursor,
    Fixed { x: f32, y: f32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub target: ZoomTarget,
    pub scale: f32,
    pub easing: String,
    #[serde(default = "default_zoom_in_ms")]
    pub zoom_in_ms: u32,
    #[serde(default = "default_zoom_out_ms")]
    pub zoom_out_ms: u32,
    #[serde(default)]
    pub layer: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cam_action: Option<crate::settings::model::CamZoomAction>,
    #[serde(default)]
    pub smart_typing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easing_out: Option<String>,
}

fn oldest_version() -> u32 {
    1
}
fn default_zoom_in_ms() -> u32 {
    350
}
fn default_zoom_out_ms() -> u32 {
    450
}

pub(crate) fn default_fade_ms() -> u32 {
    250
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub factor: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PanelPose {
    pub cx: f32,
    pub cy: f32,
    pub size: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Arrangement {
    pub screen: Option<PanelPose>,
    pub cam: Option<PanelPose>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub layout: String,
    #[serde(default = "default_layout_transition_ms")]
    pub transition_ms: u32,
    #[serde(default = "default_layout_easing")]
    pub easing: String,
    #[serde(default)]
    pub transition_out_ms: u32,
    #[serde(default = "default_layout_easing")]
    pub easing_out: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrangement: Option<Arrangement>,
}
fn default_layout_transition_ms() -> u32 {
    350
}
fn default_layout_easing() -> String {
    "smooth".into()
}

pub use crate::edit::cammove::{CameraMove, DEFAULT_CAM_ROUNDNESS};
use crate::edit::captions::Caption;
pub use crate::edit::effect::{EffectKind, EffectRegion};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    #[serde(default = "oldest_version")]
    pub version: u32,
    pub trim: Trim,
    #[serde(default)]
    pub clip_ms: u32,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    #[serde(default)]
    pub effects: Vec<EffectRegion>,
    #[serde(default)]
    pub camera_moves: Vec<CameraMove>,
    #[serde(default)]
    pub aspect: crate::export::types::Aspect,
    pub settings: crate::settings::model::Settings,
    #[serde(default)]
    pub captions: Vec<Caption>,
}
impl Default for EditDoc {
    fn default() -> Self {
        Self {
            version: DOC_VERSION,
            trim: Trim::default(),
            clip_ms: 0,
            cuts: vec![],
            zooms: vec![],
            speed: vec![],
            layout: vec![],
            effects: vec![],
            camera_moves: vec![],
            aspect: crate::export::types::Aspect::default(),
            settings: crate::settings::model::Settings::default(),
            captions: vec![],
        }
    }
}

impl EditDoc {
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = crate::process::proc::tmp_sibling(path);
        if let Err(e) = std::fs::write(&tmp, &bytes) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, path)
    }
    pub fn assign_missing_ids(&mut self) {
        let mut n = self
            .cuts
            .iter()
            .filter_map(|c| c.id.strip_prefix('c').and_then(|d| d.parse::<u32>().ok()))
            .max()
            .map_or(0, |m| m + 1);
        for c in &mut self.cuts {
            if c.id.is_empty() {
                c.id = format!("c{n}");
                n += 1;
            }
        }
    }
    pub fn load(path: &std::path::Path) -> Option<EditDoc> {
        let bytes = std::fs::read(path).ok()?;
        match serde_json::from_slice::<EditDoc>(&bytes) {
            Ok(mut doc) => {
                doc.assign_missing_ids();
                Some(doc)
            }
            Err(e) => {
                crate::process::proc::preserve_corrupt(path, &e);
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
