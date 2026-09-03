// Small read-only `FrameRenderer` accessors used only by preview commands OUTSIDE the renderer
// (`preview.rs`, `preview_track.rs`, `preview_fx.rs`) - never by the export loop, which drives
// everything through `step_camera`/`composite_at` in `mod.rs` directly. Split into its own file
// purely to keep `mod.rs` under the size budget; these are still inherent methods on
// `FrameRenderer` (Rust privacy is scoped to the defining module + its descendants, and this file
// is a child module of `render`, so the private fields below are visible here exactly as they are
// in `mod.rs`).
use super::FrameRenderer;
use crate::actions::model::ActionEvent;

impl FrameRenderer {
    /// The export background buffer (BGRA, `out_w` x `out_h`) - the same mesh/gradient the
    /// compositor draws under the screen. Exposed so the editor preview shows the exact bg.
    pub fn bg(&self) -> &[u8] { &self.bg }

    /// Whether `webcam.webm` exists on disk for this recording (see the `has_webcam` field).
    /// Exposed so preview commands outside the renderer (`preview_fx_overlay`) can gate the
    /// spotlight's camera-exclusion hole the same way `composite_at`/`fx_state::render` do.
    pub fn has_webcam(&self) -> bool { self.has_webcam }

    /// Click (mouse-down) events mapped to output time + screen-content fraction, so the
    /// editor preview can draw click ripples matching the export. `t` is output ms; `x`/`y`
    /// are 0..1 of the screen. The output time inverts `step_camera`'s `ev_t = t - events_ms`
    /// (the camera track is sampled at `video_start + t_out`).
    pub fn click_track(&self, video_start: u64) -> Vec<(u32, f32, f32)> {
        self.cursor.clicks().into_iter().filter_map(|(et, x, y)| {
            let out = et as i64 + self.events_ms as i64 - video_start as i64;
            (out >= 0).then_some((out as u32, x, y))
        }).collect()
    }

    /// The event-time base (`events_ms`), so preview commands outside the renderer can map an
    /// event timestamp to output time the same way `click_track` does.
    pub fn events_ms(&self) -> u64 { self.events_ms }

    /// The recorded action log (hotkey hold starts/ends), so preview commands can surface
    /// recorded effect holds (e.g. spotlight) the way `click_track` surfaces clicks.
    pub fn actions(&self) -> &[ActionEvent] { &self.actions }

    /// This preset's `Scene`, resolved from the renderer's own appearance + dims (the same
    /// resolve `LayoutTrack` performs) - lets `preview_layouts` cross-fade between presets.
    pub fn resolve_layout(&self, id: crate::actions::model::LayoutId) -> crate::export::scene::Scene {
        let (ma, ow, oh) = (self.settings.appearance.for_id(id), self.layout.out_w, self.layout.out_h);
        crate::export::scene::resolve(id, &crate::settings::appearance::layout_for(ma, ow, oh),
            &crate::settings::appearance::overlay_for(ma, ow, oh, true), self.sw, self.sh)
    }

    /// One `LayoutSeg`'s resolved `Scene` - its own poses when it carries an `arrangement`, else
    /// its preset's `Scene` - through `scene::layout::resolve_seg_scene`, the exact function
    /// `LayoutTrack` uses per segment when the export runs. Lets `preview_layouts` report a posed
    /// segment's true panels with no second pose-math path to drift out of sync with the export.
    pub fn resolve_seg(&self, seg: &crate::edit::model::LayoutSeg) -> crate::export::scene::Scene {
        crate::export::scene::layout::resolve_seg_scene(seg, &self.settings.appearance,
            self.layout.out_w, self.layout.out_h, self.sw, self.sh)
    }
}
