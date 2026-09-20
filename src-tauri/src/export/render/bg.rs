use super::{render_edit::EditState, FrameRenderer};
use crate::export::scene::background;
use crate::session::paths::ProjectPaths;
use crate::settings::background::BackgroundSettings;

pub(super) const BG_MESH: &[u8] = include_bytes!("../../../assets/backgrounds/bg.jpg");

pub(super) fn build_bg(
    paths: &ProjectPaths,
    settings: &BackgroundSettings,
    w: u32,
    h: u32,
) -> Vec<u8> {
    background::build(settings, BG_MESH, w, h, &paths.folder)
}

impl FrameRenderer {
    pub fn reload_edit(&mut self, paths: &ProjectPaths) {
        let es = EditState::load(
            paths,
            &self.actions,
            &self.layout,
            self.sw,
            self.sh,
            self.events_ms as i64 - self.video_start as i64,
            self.video_start,
            self.full_dur_ms,
        );
        if es.settings.background != self.settings.background {
            self.bg = build_bg(
                paths,
                &es.settings.background,
                self.layout.out_w,
                self.layout.out_h,
            );
            self.compositor.set_bg_dynamic(
                background::video_source(&es.settings.background, &paths.folder).is_some(),
            );
        }
        self.cursor
            .set_smoothness(es.settings.cursor.smoothness_at(self.os_cursor_in_video));
        self.cursor
            .set_idealize(es.settings.cursor.idealize_at(self.os_cursor_in_video));
        self.cursor
            .set_tilt(es.settings.cursor.tilt_at(self.os_cursor_in_video));
        self.settings = es.settings;
        self.cfg = es.cfg;
        self.track = es.track;
        self.regions = es.regions;
        self.effects = es.effects;
        self.cam_moves = es.cam_moves;
        self.map = es.map;
        self.clip_mix = es.clip_mix;
        self.captions = es.captions;
        self.grade = es.grade;
        self.texts = es.texts;
    }

    pub fn background(&self) -> &BackgroundSettings {
        &self.settings.background
    }

    pub fn swap_bg(&mut self, buf: Vec<u8>) -> Vec<u8> {
        std::mem::replace(&mut self.bg, buf)
    }
}
