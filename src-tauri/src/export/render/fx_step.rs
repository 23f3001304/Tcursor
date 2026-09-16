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
        let masks = crate::export::fx::fx_masks::masks_at(
            &self.effects,
            &pose.scene,
            pose.cam,
            crate::export::coordmap::full_src(self.sw, self.sh),
            ow,
            oh,
            pose.out_t,
            self.settings.clickfx.spotlight_dim,
        );
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
            masks,
            self.grade,
        );
        crate::export::fx::textdraw::overlay(
            out,
            ow,
            oh,
            &self.texts,
            self.settings.ui.accent,
            pose.out_t,
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
