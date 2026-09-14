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

    /// The take's source spans (`render::spans`) - one full-canvas span unless a mid-take display
    /// switch cropped it. Exposed so `preview_layouts` hands the editor the very rects the export
    /// crops by, instead of the editor deriving a second set from `sync.json`.
    pub fn spans(&self) -> &[crate::export::render::spans::SourceSpan] { self.track.spans() }

    /// How much smaller (w, h) a span's screen panel is than the full-canvas one, as ratios about
    /// the panel centre - `inset_rect` at the span's own size over `inset_rect` at the canvas's.
    /// The live TS preview scales its layout-resolved screen rect by this to give a switched-to
    /// display its own aspect without re-running any pose math of its own; it is exact for the
    /// inset-based presets, and the paused stage shows the export's own frame either way.
    pub fn span_fit(&self, src: crate::export::types::RectF) -> (f32, f32) {
        let base = crate::export::coordmap::inset_rect(self.sw, self.sh, &self.layout);
        let s = crate::export::coordmap::inset_rect(src.w.max(1.0) as u32, src.h.max(1.0) as u32, &self.layout);
        (s.2 as f32 / base.2.max(1) as f32, s.3 as f32 / base.3.max(1) as f32)
    }

    /// The clip-to-output clock map built from the doc's trim, cuts and speed spans.
    pub fn time_map(&self) -> &crate::export::remap::TimeMap { &self.map }

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

    /// The reference inset width `cursorset::draw` normalizes the synthetic cursor's on-screen
    /// size against (as a fraction of `out_w`) - the width `inset_rect` computes for this
    /// renderer's OWN base `Layout` (fixed pad/scale, NOT a per-preset appearance value: see
    /// `composite_at`'s `inset_rect(self.sw, self.sh, &self.layout)` call), so a screen panel
    /// narrower than this reference gets a proportionally smaller cursor - exactly like a small
    /// PiP screen shrinks it in the export. Exposed so `preview_layouts` can report it alongside
    /// the panel rects, letting the editor preview compute the SAME `panel` scale factor
    /// (`cursorPanel.ts`'s `panelFactor`) the export's `cursorset::draw` does, with no second
    /// formula to drift out of sync (this is a plain field read + the already-tested `inset_rect`
    /// - no new logic to test here).
    pub fn inset_w_frac(&self) -> f32 {
        crate::export::coordmap::inset_rect(self.sw, self.sh, &self.layout).2 as f32
            / self.layout.out_w.max(1) as f32
    }
}
