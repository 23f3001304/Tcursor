use super::FrameRenderer;
use crate::actions::model::ActionEvent;

impl FrameRenderer {
    pub fn bg(&self) -> &[u8] {
        &self.bg
    }

    pub fn has_webcam(&self) -> bool {
        self.has_webcam
    }

    pub fn click_track(&self, video_start: u64) -> Vec<(u32, f32, f32)> {
        self.cursor
            .clicks()
            .into_iter()
            .filter_map(|(et, x, y)| {
                let out = et as i64 + self.events_ms as i64 - video_start as i64;
                (out >= 0).then_some((out as u32, x, y))
            })
            .collect()
    }

    pub fn events_ms(&self) -> u64 {
        self.events_ms
    }

    pub fn spans(&self) -> &[crate::export::render::spans::SourceSpan] {
        self.track.spans()
    }

    pub fn span_fit(&self, src: crate::export::types::RectF) -> (f32, f32) {
        let base = crate::export::coordmap::inset_rect(self.sw, self.sh, &self.layout);
        let s = crate::export::coordmap::inset_rect(
            src.w.max(1.0) as u32,
            src.h.max(1.0) as u32,
            &self.layout,
        );
        (
            s.2 as f32 / base.2.max(1) as f32,
            s.3 as f32 / base.3.max(1) as f32,
        )
    }

    pub fn time_map(&self) -> &crate::export::remap::TimeMap {
        &self.map
    }

    pub fn actions(&self) -> &[ActionEvent] {
        &self.actions
    }

    pub fn resolve_layout(
        &self,
        id: crate::actions::model::LayoutId,
    ) -> crate::export::scene::Scene {
        let (ma, ow, oh) = (
            self.settings.appearance.for_id(id),
            self.layout.out_w,
            self.layout.out_h,
        );
        crate::export::scene::resolve(
            id,
            &crate::settings::appearance::layout_for(ma, ow, oh),
            &crate::settings::appearance::overlay_for(ma, ow, oh, true),
            self.sw,
            self.sh,
        )
    }

    pub fn resolve_seg(&self, seg: &crate::edit::model::LayoutSeg) -> crate::export::scene::Scene {
        crate::export::scene::layout::resolve_seg_scene(
            seg,
            &self.settings.appearance,
            self.layout.out_w,
            self.layout.out_h,
            self.sw,
            self.sh,
        )
    }

    pub fn inset_w_frac(&self) -> f32 {
        crate::export::coordmap::inset_rect(self.sw, self.sh, &self.layout).2 as f32
            / self.layout.out_w.max(1) as f32
    }
}
