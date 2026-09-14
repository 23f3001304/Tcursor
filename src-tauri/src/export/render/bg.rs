// The renderer's background: how the static `bg` buffer is built, how an edit refreshes the
// edit.json-derived state around it, and how a VIDEO background's decoded frames get swapped in.
//
// Split out of `render/mod.rs` (which was at its line budget) rather than grown there. A child
// module can implement methods on its parent's type and reach its private fields, so this is the
// same `FrameRenderer` - only the file boundary moved.
use crate::export::scene::background;
use crate::session::paths::ProjectPaths;
use crate::settings::background::BackgroundSettings;
use super::{render_edit::EditState, FrameRenderer};

/// The legacy bundled background ("Classic"): what `BackgroundSettings.mesh == ""` renders, which
/// is what every project saved before the wallpaper library deserializes to.
pub(super) const BG_MESH: &[u8] = include_bytes!("../../../assets/backgrounds/bg.jpg");

/// Build the static background buffer for `settings` at `w`x`h`. `paths.folder` is the project
/// folder an `Image`/`Video` asset path is relative to (assets are never stored absolute).
pub(super) fn build_bg(paths: &ProjectPaths, settings: &BackgroundSettings, w: u32, h: u32) -> Vec<u8> {
    background::build(settings, BG_MESH, w, h, &paths.folder)
}

impl FrameRenderer {
    /// Refresh the `edit.json`-derived state in place (zoom/layout/regions/effects/cam_moves), WITHOUT
    /// recreating the GPU compositor or re-probing dims. Also rebuilds `bg`, but ONLY when background
    /// settings changed (the decode shells out to ffmpeg - every OTHER edit must stay cheap).
    pub fn reload_edit(&mut self, paths: &ProjectPaths) {
        let es = EditState::load(paths, &self.actions, &self.layout, self.sw, self.sh,
            self.events_ms as i64 - self.video_start as i64, self.video_start, self.full_dur_ms);
        if es.settings.background != self.settings.background {
            self.bg = build_bg(paths, &es.settings.background, self.layout.out_w, self.layout.out_h);
            // A background that MOVES has to re-upload every frame; one that does not must keep
            // uploading only on change. Re-asserted here because an edit can flip either way.
            self.compositor.set_bg_dynamic(background::video_source(&es.settings.background, &paths.folder).is_some());
        }
        // Live-apply Smoothness/idealize; the `_at` variants also flip the path to RAW the moment
        // the style is switched to System on a recording with no baked OS cursor.
        self.cursor.set_smoothness(es.settings.cursor.smoothness_at(self.os_cursor_in_video));
        self.cursor.set_idealize(es.settings.cursor.idealize_at(self.os_cursor_in_video));
        self.cursor.set_tilt(es.settings.cursor.tilt_at(self.os_cursor_in_video));
        self.settings = es.settings; self.cfg = es.cfg; self.track = es.track;
        self.regions = es.regions; self.effects = es.effects; self.cam_moves = es.cam_moves; self.map = es.map;
    }

    /// This renderer's background settings - what the exporter asks before opening a decode stream.
    pub fn background(&self) -> &BackgroundSettings { &self.settings.background }

    /// Swap in one decoded VIDEO background frame, handing back the buffer it replaces so the
    /// caller can return it to its pool. A move, not a copy: at 1080p this runs 60 times a second
    /// and each buffer is ~8 MB.
    pub fn swap_bg(&mut self, buf: Vec<u8>) -> Vec<u8> { std::mem::replace(&mut self.bg, buf) }
}
