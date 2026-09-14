//! The `edit.json`-derived slice of the renderer's state, extracted so an editor edit can be
//! reflected by rebuilding ONLY this (cheap CPU work) instead of a full `FrameRenderer::new`
//! - which recreates the GPU device, decodes the background, probes the video, and preps the
//! cursor sprites. Both `new()` and `FrameRenderer::reload_edit` build it from the same code,
//! so an edit reflects identically whether the renderer was freshly built or refreshed in place.
//!
//! NOTE: cursor prep (`CursorPrep`) is deliberately NOT here - it is edit-independent for the
//! warm preview (only `composite_at` uses it, which the editor never calls) yet it dominates a
//! full build, so keeping it out is what makes `reload_edit` cheap.
use crate::actions::model::ActionEvent;
use crate::edit::model::EffectRegion;
use crate::export::camera::moves::CameraMoveTrack;
use crate::export::remap::TimeMap;
use crate::export::render::spans::{self, SpanTrack};
use crate::export::scene::layout::LayoutTrack;
use crate::export::types::{Layout, ZoomConfig, ZoomRegion};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

/// Layout cross-fade duration (ms). Shared by every `EditState::load`.
const TRANSITION_MS: u32 = 350;

/// Everything the renderer derives from `edit.json` that a zoom/spotlight edit can change: the
/// settings, zoom config, layout track, anchored zoom regions, and effect regions. Pure CPU -
/// no GPU, no video probe, no background decode, no cursor prep - so it rebuilds in ~ms.
pub(crate) struct EditState {
    pub map: TimeMap,
    pub settings: Settings,
    pub cfg: ZoomConfig,
    /// One `LayoutTrack` per SOURCE SPAN, so the screen panel takes each span's own aspect and a
    /// mid-take display switch eases between them (`render::spans`). A take that never switched
    /// has one span and one track, i.e. the plain layout track it always was.
    pub track: SpanTrack,
    pub regions: Vec<ZoomRegion>,
    pub effects: Vec<EffectRegion>,
    pub cam_moves: CameraMoveTrack,
}

impl EditState {
    /// Load the edit-derived state from `paths`. `actions`/`layout`/`sw`/`sh` are the
    /// edit-INDEPENDENT inputs the caller already holds (recorded action log, preview layout,
    /// probed video dims), so this touches only `edit.json`. `shift` is `events_ms - video_start`,
    /// needed only by the recorded-action fallback below (that log is on the EVENT clock; the
    /// track it builds is sampled on the OUTPUT clock like every other region).
    /// `full_dur_ms` is the clip's length: the `TimeMap` (trim, cuts, speed) is built from the doc and
    /// every region list is moved onto the output clock (`remap_doc`) BEFORE any track is built, which
    /// is why no evaluator knows about cuts or speed. The map rides along for the exporter's plan.
    pub(crate) fn load(paths: &ProjectPaths, actions: &[ActionEvent], layout: &Layout, sw: u32, sh: u32,
                       shift: i64, video_start: u64, full_dur_ms: u32) -> Self {
        let raw = crate::edit::seed::load_or_seed(paths);
        let map = TimeMap::build(&raw.trim, &raw.cuts, &raw.speed, full_dur_ms);
        let doc = crate::edit::remap_doc::remap_doc(&raw, &map);
        let settings = doc.settings.clone();
        let cfg = settings.zoom.to_zoom_config();
        // The recorded layout switches sit on the clip clock; move them like every region.
        let acts: Vec<ActionEvent> = crate::edit::seed::actions_on_output_clock(actions, shift).into_iter()
            .map(|a| ActionEvent { t: map.out_of(a.t), kind: a.kind }).collect();
        // One track PER SPAN: `resolve` shapes the screen panel from the source size it is handed,
        // so a span on a 16:10 display resolves a 16:10 panel and the switch eases between the two
        // resolved scenes through `Scene::lerp`. `(sw, sh)` (the whole canvas) is span 0's size, so
        // a take with no switch builds exactly one track from exactly the old arguments.
        let track = SpanTrack::build(
            spans::spans_for(paths, (sw, sh), video_start, &map),
            |ssw, ssh| if doc.layout.is_empty() {
                LayoutTrack::new(&acts, &settings.appearance, layout.out_w, layout.out_h, ssw, ssh, TRANSITION_MS)
            } else {
                LayoutTrack::from_segs(&doc.layout, &settings.appearance, layout.out_w, layout.out_h, ssw, ssh)
            });
        // RAW canvas-space anchors: `step_camera` re-anchors them into each frame's own screen
        // panel (`layout::anchor_frame`), so a layout transition mid-zoom carries the aim with it.
        let regions = crate::export::render::fromedit::regions_from_doc(&doc, sw, sh);
        let cam_moves = CameraMoveTrack::from_doc(&doc.camera_moves);
        EditState { map, settings, cfg, track, regions, effects: doc.effects.clone(), cam_moves }
    }
}

#[cfg(test)]
mod tests {
    use crate::export::render::{FrameRenderer, OUT_FPS, OUT_STEP_MS};
    use crate::export::settings::Resolution;
    use crate::export::types::Layout;
    use crate::session::paths::ProjectPaths;

    /// A bit-exact signature of the camera curve, so two renderers can be compared byte-for-byte.
    fn sweep(r: &mut FrameRenderer, vs: u64) -> Vec<[u32; 5]> {
        r.reset_camera();
        (0..=120u64).map(|k| {
            let p = r.step_camera(vs + k * 1000 / OUT_FPS, (k * 1000 / OUT_FPS) as u32, OUT_STEP_MS);
            [p.cam.scale.to_bits(), p.cam.cx.to_bits(), p.cam.cy.to_bits(),
             p.scene.screen.rect.x.to_bits(), p.scene.screen.rect.w.to_bits()]
        }).collect()
    }

    /// `reload_edit` must leave the camera curve identical to a full rebuild (so an edit reflects
    /// exactly) - and be far cheaper. Ignored: needs a real recording folder. Run:
    ///   TCURSOR_REC=<folder> cargo test --lib export::render::render_edit::tests::reload -- --ignored --nocapture
    #[test]
    #[ignore]
    fn reload_edit_matches_full_rebuild_and_is_faster() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let paths = ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let t0 = std::time::Instant::now();
        let (mut r1, m) = FrameRenderer::new(&paths, Layout::default(), 60, Resolution::Source, None).expect("build r1");
        let build = t0.elapsed();
        let (mut r2, _) = FrameRenderer::new(&paths, Layout::default(), 60, Resolution::Source, None).expect("build r2");
        let baseline = sweep(&mut r2, m.video_start);
        assert_eq!(sweep(&mut r1, m.video_start), baseline, "two fresh builds diverged");
        let t1 = std::time::Instant::now();
        r1.reload_edit(&paths);
        let reload = t1.elapsed();
        assert_eq!(sweep(&mut r1, m.video_start), baseline, "reload_edit diverged from a full rebuild");
        eprintln!("reload_edit: build={build:?} reload={reload:?} (reload should be MUCH faster)");
    }
}
