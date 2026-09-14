use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle { System, Enhanced, Hidden }
impl CursorStyle {
    /// LEGACY DETECTION ONLY. Recording no longer bakes the OS cursor into the capture for any
    /// style (`recorder.rs` passes `with_cursor: false` and records the real cursor as its own
    /// layer instead), so this is not a capture decision any more - it is how a recording made
    /// BEFORE that change is recognized: back then, and only back then, `System` meant baked
    /// pixels. Always paired with a "does this project have a cursor layer" check; see
    /// `settings::store::os_cursor_in_video`.
    pub fn captures_os_cursor(self) -> bool { matches!(self, CursorStyle::System) }
}

/// The glass shape drawn BEHIND the cursor, whatever pack it comes from. `None` is today's look
/// (nothing behind the sprite); `Glass` adds a refracting disc that morphs by cursor kind - a
/// vertical pill over text, stretching into a selection bar while the left button is held there.
/// Independent of the pack's own `material`: a plain pack can have a glass back, and a glass pack
/// can have none.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CursorBack {
    #[default]
    None,
    Glass,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,             // scale of the base cursor size (1.0 = default)
    pub smoothness: f32,       // 0..1 cursor-follow glide (0 = snappy/raw, 1 = glassy); the low-pass alpha
    pub path_idealize: f32,    // 0..1 straighten wandering paths into clean strokes between clicks (0 = off)
    pub motion_blur: f32,      // 0..1 trail strength (0 = off)
    /// 0..1 motion lean: how far a fast cursor tips into its own travel, and overshoots once
    /// coming back upright when it stops (`export/cursor/tilt.rs`). Scales the 6-degree cap;
    /// 0 switches the filter off entirely. Field-level default so a `cursor` block written before
    /// this existed still gains the feature (the struct-level `default` only covers a MISSING block).
    #[serde(default = "default_tilt")]
    pub tilt: f32,
    pub click_bounce: bool,
    pub bounce_intensity: f32, // 0..1 dip depth (0.5 = ~0.18 dip, 1.0 = 0.36 dip)
    pub pack: String,          // "default" (built-in) or an imported id; see export/cursor/pack.rs
    pub back: CursorBack,      // the glass shape behind the cursor; see `CursorBack`
}
/// `tilt`'s default: present, but at barely over a third of the cap - the owner asked for "very
/// subtle", and a lean you notice as a lean is already too much.
fn default_tilt() -> f32 { 0.35 }
impl Default for CursorSettings {
    fn default() -> Self { Self { style: CursorStyle::System, size: 1.0, smoothness: 0.6, path_idealize: 0.0, motion_blur: 0.35, tilt: default_tilt(), click_bounce: true, bounce_intensity: 0.5, pack: "default".to_string(), back: CursorBack::None } }
}
impl CursorSettings {
    /// `System` on a recording whose video has NO baked OS cursor: draw the synthetic cursor as
    /// plainly as possible instead of nothing. `captures_os_cursor` is a RECORD-time property, so
    /// `os_cursor_in_video` must come from the record-time snapshot, never from this (editable) doc.
    pub fn plain_os(&self, os_cursor_in_video: bool) -> bool {
        self.style == CursorStyle::System && !os_cursor_in_video
    }
    /// `smoothness` (the glide `export/cursor/path.rs` draws between the recording's rests), or
    /// 0.0 (the raw recorded path) in plain-OS mode.
    pub fn smoothness_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) { 0.0 } else { self.smoothness.clamp(0.0, 1.0) }
    }
    /// `path_idealize`, or 0.0 (no straightening) in plain-OS mode.
    pub fn idealize_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) { 0.0 } else { self.path_idealize }
    }
    /// `tilt`, or 0.0 in plain-OS mode: the lean is fake polish, and a re-created system cursor
    /// promises none of it - the same rule `follow_alpha_at` and `idealize_at` apply to the path.
    pub fn tilt_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) { 0.0 } else { self.tilt }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_back_is_off_by_default_and_round_trips() {
        assert_eq!(CursorSettings::default().back, CursorBack::None);
        let json = serde_json::to_string(&CursorBack::Glass).unwrap();
        assert_eq!(json, "\"glass\"", "the wire name the TS `CursorBackStyle` mirrors");
        assert_eq!(serde_json::from_str::<CursorBack>("\"none\"").unwrap(), CursorBack::None);
    }

    #[test]
    fn the_motion_tilt_round_trips_and_defaults_present_but_subtle() {
        assert_eq!(CursorSettings::default().tilt, 0.35);
        let json = serde_json::to_string(&CursorSettings { tilt: 0.8, ..Default::default() }).unwrap();
        assert!(json.contains("\"tilt\":0.8"), "the wire name the TS mirror reads: {json}");
        assert_eq!(serde_json::from_str::<CursorSettings>(&json).unwrap().tilt, 0.8);
        // A `cursor` block saved before the field existed keeps every other value and gains the
        // default lean - the field-level `#[serde(default)]`, not the struct-level one.
        let old = r#"{"style":"enhanced","size":1.0,"smoothness":0.2,"pack":"crystal"}"#;
        let s: CursorSettings = serde_json::from_str(old).unwrap();
        assert_eq!((s.tilt, s.smoothness), (0.35, 0.2));
        // Plain-OS mode has no fake polish at all, whatever the doc asks for.
        let sys = CursorSettings { style: CursorStyle::System, tilt: 1.0, ..Default::default() };
        assert_eq!(sys.tilt_at(false), 0.0);
        assert_eq!(sys.tilt_at(true), 1.0);
    }

    #[test]
    fn a_settings_file_written_before_the_back_existed_still_loads() {
        // `#[serde(default)]` on the struct: every pre-2026-09-14 settings.json omits `back`, and
        // must come back as "no back" rather than failing the whole doc load.
        let old = r#"{"style":"enhanced","size":1.0,"pack":"crystal"}"#;
        let s: CursorSettings = serde_json::from_str(old).unwrap();
        assert_eq!(s.back, CursorBack::None);
        assert_eq!(s.pack, "crystal");
    }
}
