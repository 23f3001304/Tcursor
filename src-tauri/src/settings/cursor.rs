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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,             // scale of the base cursor size (1.0 = default)
    pub smoothness: f32,       // 0..1 cursor-follow glide (0 = snappy/raw, 1 = glassy); the low-pass alpha
    pub path_idealize: f32,    // 0..1 straighten wandering paths into clean strokes between clicks (0 = off)
    pub motion_blur: f32,      // 0..1 trail strength (0 = off)
    pub click_bounce: bool,
    pub bounce_intensity: f32, // 0..1 dip depth (0.5 = ~0.18 dip, 1.0 = 0.36 dip)
    pub pack: String,          // "default" (built-in) or an imported id; see export/cursor/pack.rs
}
impl Default for CursorSettings {
    fn default() -> Self { Self { style: CursorStyle::System, size: 1.0, smoothness: 0.6, path_idealize: 0.0, motion_blur: 0.35, click_bounce: true, bounce_intensity: 0.5, pack: "default".to_string() } }
}
impl CursorSettings {
    /// Low-pass alpha for `Cursor::at` derived from `smoothness`: 0 -> snappy (0.75, follows closely),
    /// 1 -> glassy glide (0.10). Default 0.6 -> ~0.36, matching the old hardcoded 0.35.
    pub fn follow_alpha(&self) -> f32 { 0.75 - 0.65 * self.smoothness.clamp(0.0, 1.0) }

    /// `System` on a recording whose video has NO baked OS cursor: draw the synthetic cursor as
    /// plainly as possible instead of nothing. `captures_os_cursor` is a RECORD-time property, so
    /// `os_cursor_in_video` must come from the record-time snapshot, never from this (editable) doc.
    pub fn plain_os(&self, os_cursor_in_video: bool) -> bool {
        self.style == CursorStyle::System && !os_cursor_in_video
    }
    /// `follow_alpha`, or 1.0 (the raw recorded path, no glide) in plain-OS mode.
    pub fn follow_alpha_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) { 1.0 } else { self.follow_alpha() }
    }
    /// `path_idealize`, or 0.0 (no straightening) in plain-OS mode.
    pub fn idealize_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) { 0.0 } else { self.path_idealize }
    }
}
