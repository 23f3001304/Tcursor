use serde::{Deserialize, Serialize};

/// Current `EditDoc` schema version. v2 is the one-clock contract: EVERY region list in the doc
/// (`zooms`, `effects`, `layout`, `camera_moves`) is on the OUTPUT clock (0 = first video frame).
/// v1 docs stored `effects`/`layout` on the raw EVENT clock; `seed::load_or_seed` migrates them.
pub const DOC_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct Trim { pub in_ms: u32, pub out_ms: u32 }
impl Default for Trim { fn default() -> Self { Self { in_ms: 0, out_ms: 0 } } }
impl Trim {
    /// The effective `[in_ms, out_ms)` export/preview range against a clip of `total_dur_ms`.
    /// `out_ms == 0` (the doc-level default, "not yet set") means "no trim / whole clip"; both
    /// bounds are clamped into `[0, total_dur_ms]` and `in_ms` never exceeds the resolved
    /// `out_ms`, so a degenerate/inverted range safely collapses to zero-length instead of
    /// underflowing at the call site. Export (`exporter::export`) and preview
    /// (`preview_track::camera_track`'s trim clamp, the frontend's `resolveTrim`) all read the
    /// trim through this one function, so they always agree on the effective range.
    pub fn resolve(&self, total_dur_ms: u32) -> (u32, u32) {
        let out = if self.out_ms == 0 { total_dur_ms } else { self.out_ms.min(total_dur_ms) };
        let inp = self.in_ms.min(out);
        (inp, out)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// A removed stretch of clip time. `id` is defaulted on load for docs written before cuts had one
/// (`EditDoc::assign_missing_ids`), so those keep loading unchanged.
pub struct Cut { #[serde(default)] pub id: String, pub start_ms: u32, pub end_ms: u32 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget { Cursor, Fixed { x: f32, y: f32 } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String, pub start_ms: u32, pub end_ms: u32,
    pub target: ZoomTarget, pub scale: f32, pub easing: String,
    #[serde(default = "default_zoom_in_ms")] pub zoom_in_ms: u32,
    #[serde(default = "default_zoom_out_ms")] pub zoom_out_ms: u32,
    /// Priority when this zoom overlaps another in time - higher wins. Also determines
    /// which timeline row it renders on. Auto-assigned on creation (see `edit::ops::api::auto_layer`),
    /// user-overridable via `UpdateZoom`.
    #[serde(default)] pub layer: u32,
    /// Per-zoom webcam-on-zoom override. `None` inherits the global
    /// `ZoomSettings::resolved_cam_action`, so docs written before this existed are unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cam_action: Option<crate::settings::model::CamZoomAction>,
}

fn oldest_version() -> u32 { 1 }
fn default_zoom_in_ms() -> u32 { 350 }
fn default_zoom_out_ms() -> u32 { 450 }
/// Matches `fx_state::FADE_MS` (the spotlight fade baseline).
fn default_fade_ms() -> u32 { 250 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed { pub id: String, pub start_ms: u32, pub end_ms: u32, pub factor: f32 }

/// Normalized panel pose on the output frame: center (fractions of output w/h) + `size` (panel
/// HEIGHT as a fraction of output height - the same vocabulary `CameraMove` uses). Width always
/// derives from the panel's OWN aspect at resolve time, so a pose can never stretch content.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PanelPose { pub cx: f32, pub cy: f32, pub size: f32 }

/// A custom panel arrangement: explicit poses for the screen and webcam panels of one
/// `LayoutSeg`. A `None` panel is not shown (alpha 0); the ops keep at least one panel `Some`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Arrangement { pub screen: Option<PanelPose>, pub cam: Option<PanelPose> }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg {
    pub id: String, pub start_ms: u32, pub end_ms: u32, pub layout: String,
    /// Cross-fade duration (ms) INTO this layout - the editable transition feel.
    #[serde(default = "default_layout_transition_ms")] pub transition_ms: u32,
    /// Easing wire-name for the fade ("linear" | "smooth" | "spring").
    #[serde(default = "default_layout_easing")] pub easing: String,
    /// Cross-fade duration (ms) OUT of this layout, COMPLETING at `end_ms` (symmetric with the
    /// entry, which starts at `start_ms`). `0` - the default, and what every pre-existing doc
    /// deserializes to - is today's hard cut.
    #[serde(default)] pub transition_out_ms: u32,
    /// Easing wire-name for the exit fade; only meaningful when `transition_out_ms > 0`.
    #[serde(default = "default_layout_easing")] pub easing_out: String,
    /// Explicit panel poses for this segment. `None` - what every doc written before this
    /// existed loads as, and what `skip_serializing_if` keeps out of a re-saved old doc -
    /// resolves from the `layout` preset exactly as before. `Some` WINS over the preset and
    /// makes `layout` display-only provenance (it still selects the appearance block the
    /// panels' radius/ring/shape come from). See `export::scene::arrangement`.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arrangement: Option<Arrangement>,
}
fn default_layout_transition_ms() -> u32 { 350 }
fn default_layout_easing() -> String { "smooth".into() }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraMove {
    pub id: String, pub t_ms: u32, pub x: f32, pub y: f32, pub size: f32,
    #[serde(default = "default_cam_easing")] pub easing: String,
}
fn default_cam_easing() -> String { "smooth".into() }

/// An editable effect region on the timeline. v1 covers Spotlight; the kind grows over phases.
/// Params default from settings for now (per-region overrides are a later addition).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion {
    pub id: String, pub kind: EffectKind, pub start_ms: u32, pub end_ms: u32,
    #[serde(default = "default_fade_ms")] pub fade_in_ms: u32,
    #[serde(default = "default_fade_ms")] pub fade_out_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub mode: Option<crate::settings::model::SpotlightMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub dim: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub feather: Option<f32>,
    /// Priority when this region overlaps another Spotlight region - higher wins.
    #[serde(default)] pub layer: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    /// Missing on disk means the OLDEST schema, never the current one: the app has always written
    /// this field, so a doc without it predates versioning and must still be offered to `migrate`.
    /// (Without this field default the container `#[serde(default)]` would fill in `DOC_VERSION`
    /// and silently skip every migration.) A doc built in memory starts at `DOC_VERSION`.
    #[serde(default = "oldest_version")] pub version: u32,
    pub trim: Trim,
    /// The recording's TRUE full duration (ms), independent of `trim`. The upper bound every
    /// region-placing op (`AddZoom`, `AddLayoutSeg`, `AddCameraMove`, ...) clamps against via
    /// `edit::ops::region::dur_bound` - so trimming the clip no longer collapses a NEW region to
    /// the trim point. `0` means "not yet known"; `seed`/`migrate` always backfill it.
    #[serde(default)] pub clip_ms: u32,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    #[serde(default)]
    pub effects: Vec<EffectRegion>,
    #[serde(default)]
    pub camera_moves: Vec<CameraMove>,
    /// Output frame aspect ratio; `Aspect::Source` (the default) matches today's behavior
    /// exactly, so a doc saved before this field existed loads unchanged. See `Layout::apply_aspect`.
    #[serde(default)]
    pub aspect: crate::export::types::Aspect,
    pub settings: crate::settings::model::Settings,
}
impl Default for EditDoc {
    fn default() -> Self {
        Self { version: DOC_VERSION, trim: Trim::default(), clip_ms: 0, cuts: vec![], zooms: vec![], speed: vec![], layout: vec![], effects: vec![], camera_moves: vec![],
            aspect: crate::export::types::Aspect::default(), settings: crate::settings::model::Settings::default() }
    }
}

impl EditDoc {
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = crate::win::sys::proc::tmp_sibling(path);
        if let Err(e) = std::fs::write(&tmp, &bytes) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, path)
    }
    /// Cuts saved before they had ids get `c{n}` ones, `n` counting up from the highest live id.
    pub fn assign_missing_ids(&mut self) {
        let mut n = self.cuts.iter().filter_map(|c| c.id.strip_prefix('c').and_then(|d| d.parse::<u32>().ok())).max().map_or(0, |m| m + 1);
        for c in &mut self.cuts { if c.id.is_empty() { c.id = format!("c{n}"); n += 1; } }
    }
    pub fn load(path: &std::path::Path) -> Option<EditDoc> {
        let bytes = std::fs::read(path).ok()?;
        match serde_json::from_slice::<EditDoc>(&bytes) {
            Ok(mut doc) => { doc.assign_missing_ids(); Some(doc) }
            Err(e) => {
                crate::win::sys::proc::preserve_corrupt(path, &e);
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
