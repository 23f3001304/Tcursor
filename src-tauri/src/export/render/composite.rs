use super::screen_mix;
use super::{FramePose, FrameRenderer};
use crate::export::coordmap::inset_rect;

impl FrameRenderer {
    pub fn composite_at(
        &mut self,
        pose: &FramePose,
        screen: &[u8],
        prev: Option<&[u8]>,
        webcam: Option<(&[u8], u32, u32)>,
        out: &mut Vec<u8>,
    ) {
        let (ow, oh) = (self.layout.out_w, self.layout.out_h);
        let screen = match (pose.mix, prev) {
            (Some(m), Some(p)) => {
                screen_mix::blend_into(
                    &mut self.mix_buf,
                    screen,
                    p,
                    self.sw,
                    self.sh,
                    m.prev_src,
                    pose.scene.src,
                    m.alpha,
                );
                &self.mix_buf[..]
            }
            _ => screen,
        };
        self.compositor.composite_into(
            screen,
            self.sw,
            self.sh,
            webcam,
            pose.cam,
            &self.bg,
            &self.layout,
            &pose.scene,
            out,
        );
        let inset_w = inset_rect(self.sw, self.sh, &self.layout).2 as f32;
        let sc = &pose.scene.screen;
        let captured = crate::export::cursor::captured::draws_captured(
            self.settings.cursor.style,
            self.captured.is_some(),
        );
        let lens = (!captured)
            .then(|| self.lenses(pose, ow, oh, sc, inset_w))
            .flatten();
        self.fx_pass(pose, out, ow, oh, lens);
        if captured {
            let src_w = pose.scene.src.w.max(1.0) as u32;
            if let Some(cc) = &self.captured {
                cc.draw(
                    out, ow, oh, pose.cur, pose.cam, sc, inset_w, src_w, pose.ev_t,
                );
            }
        } else if let Some(cp) = &mut self.cprep {
            crate::export::cursor::cursorset::draw(
                cp,
                out,
                ow,
                oh,
                pose.cur,
                pose.cam,
                sc,
                inset_w,
                pose.ev_t,
                pose.out_t,
                &self.settings.cursor,
                self.os_cursor_in_video,
                self.cursor.tilt_deg(),
            );
        }
    }

    fn lenses(
        &self,
        pose: &FramePose,
        ow: u32,
        oh: u32,
        sc: &crate::export::scene::Panel,
        inset_w: f32,
    ) -> Option<crate::export::fx::fx_lens::Lenses> {
        use crate::export::fx::fx_lensbuild::{lenses_at, wants_lens, LensFrame};
        let c = &self.settings.cursor;
        let plain_os = c.plain_os(self.os_cursor_in_video);
        if !wants_lens(self.cprep.as_ref(), c, plain_os) {
            return None;
        }
        let info = self.cursor.screen();
        let f = LensFrame {
            cur: pose.cur,
            cam: pose.cam,
            ow,
            oh,
            screen: sc,
            inset_w,
            info: &info,
            src: pose.scene.src,
            ev_t: pose.ev_t,
            out_t: pose.out_t,
            tilt_deg: self.cursor.tilt_deg(),
        };
        lenses_at(self.cprep.as_ref()?, c, f, plain_os)
    }
}
