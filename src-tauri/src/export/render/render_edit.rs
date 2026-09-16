use crate::actions::model::ActionEvent;
use crate::edit::captions::Caption;
use crate::edit::model::EffectRegion;
use crate::export::camera::moves::CameraMoveTrack;
use crate::export::remap::TimeMap;
use crate::export::render::spans::{self, SpanTrack};
use crate::export::scene::layout::LayoutTrack;
use crate::export::types::{Layout, ZoomConfig, ZoomRegion};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

const TRANSITION_MS: u32 = 350;

pub(crate) struct EditState {
    pub map: TimeMap,
    pub settings: Settings,
    pub cfg: ZoomConfig,
    pub track: SpanTrack,
    pub regions: Vec<ZoomRegion>,
    pub effects: Vec<EffectRegion>,
    pub captions: Vec<Caption>,
    pub texts: Vec<crate::edit::text::TextItem>,
    pub cam_moves: CameraMoveTrack,
    pub grade: Option<crate::export::grade::GradeParams>,
}

impl EditState {
    pub(crate) fn load(
        paths: &ProjectPaths,
        actions: &[ActionEvent],
        layout: &Layout,
        sw: u32,
        sh: u32,
        shift: i64,
        video_start: u64,
        full_dur_ms: u32,
    ) -> Self {
        let raw = crate::edit::seed::load_or_seed(paths);
        let map = TimeMap::build(&raw.trim, &raw.cuts, &raw.speed, &raw.clips, full_dur_ms);
        let doc = crate::edit::remap_doc::remap_doc(&raw, &map);
        let settings = doc.settings.clone();
        let cfg = settings.zoom.to_zoom_config();
        let grade = crate::export::grade::params_of(&settings.grade);
        let acts: Vec<ActionEvent> = crate::edit::seed::actions_on_output_clock(actions, shift)
            .into_iter()
            .map(|a| ActionEvent {
                t: map.out_of(a.t),
                kind: a.kind,
            })
            .collect();
        let track = SpanTrack::build(
            spans::spans_for(paths, (sw, sh), video_start, &map),
            |ssw, ssh| {
                if doc.layout.is_empty() {
                    LayoutTrack::new(
                        &acts,
                        &settings.appearance,
                        layout.out_w,
                        layout.out_h,
                        ssw,
                        ssh,
                        TRANSITION_MS,
                    )
                } else {
                    LayoutTrack::from_segs(
                        &doc.layout,
                        &settings.appearance,
                        layout.out_w,
                        layout.out_h,
                        ssw,
                        ssh,
                    )
                }
            },
        );
        let regions = crate::export::render::fromedit::regions_from_doc(&doc, sw, sh);
        let cam_moves = CameraMoveTrack::from_doc(&doc.camera_moves);
        EditState {
            map,
            settings,
            cfg,
            track,
            regions,
            effects: doc.effects.clone(),
            captions: doc.captions.clone(),
            texts: doc.texts.clone(),
            cam_moves,
            grade,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::export::render::{FrameRenderer, OUT_FPS, OUT_STEP_MS};
    use crate::export::settings::Resolution;
    use crate::export::types::Layout;
    use crate::session::paths::ProjectPaths;

    fn sweep(r: &mut FrameRenderer, vs: u64) -> Vec<[u32; 5]> {
        r.reset_camera();
        (0..=120u64)
            .map(|k| {
                let p = r.step_camera(
                    vs + k * 1000 / OUT_FPS,
                    (k * 1000 / OUT_FPS) as u32,
                    OUT_STEP_MS,
                );
                [
                    p.cam.scale.to_bits(),
                    p.cam.cx.to_bits(),
                    p.cam.cy.to_bits(),
                    p.scene.screen.rect.x.to_bits(),
                    p.scene.screen.rect.w.to_bits(),
                ]
            })
            .collect()
    }

    #[test]
    #[ignore]
    fn reload_edit_matches_full_rebuild_and_is_faster() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let paths = ProjectPaths {
            folder: std::path::PathBuf::from(&folder),
        };
        let t0 = std::time::Instant::now();
        let platform = crate::platform::current();
        let (mut r1, m) = FrameRenderer::new(
            &paths,
            Layout::default(),
            60,
            Resolution::Source,
            None,
            platform.system.as_ref(),
        )
        .expect("build r1");
        let build = t0.elapsed();
        let (mut r2, _) = FrameRenderer::new(
            &paths,
            Layout::default(),
            60,
            Resolution::Source,
            None,
            platform.system.as_ref(),
        )
        .expect("build r2");
        let baseline = sweep(&mut r2, m.video_start);
        assert_eq!(
            sweep(&mut r1, m.video_start),
            baseline,
            "two fresh builds diverged"
        );
        let t1 = std::time::Instant::now();
        r1.reload_edit(&paths);
        let reload = t1.elapsed();
        assert_eq!(
            sweep(&mut r1, m.video_start),
            baseline,
            "reload_edit diverged from a full rebuild"
        );
        eprintln!("reload_edit: build={build:?} reload={reload:?} (reload should be MUCH faster)");
    }
}
