use super::{FramePose, FrameRenderer};
use crate::export::fx::fx_state;

impl FrameRenderer {
    pub(super) fn fx_pass(
        &mut self,
        pose: &FramePose,
        out: &mut [u8],
        ow: u32,
        oh: u32,
        lens: Option<crate::export::fx::fx_lens::Lenses>,
    ) {
        fx_state::render(
            &*self.fx,
            out,
            ow,
            oh,
            &self.settings.clickfx,
            self.cursor.events(),
            &self.actions,
            &self.effects,
            &pose.scene,
            pose.cam,
            pose.cur,
            &self.cursor.screen(),
            self.has_webcam,
            pose.out_t,
            pose.ev_t,
            &self.settings.hotkeys,
            &mut self.spot_sim,
            lens,
        );
        crate::export::fx::captiondraw::overlay(
            out,
            ow,
            oh,
            &self.captions,
            &self.settings.captions,
            self.settings.ui.accent,
            pose.out_t,
        );
    }
}
