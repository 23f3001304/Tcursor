use crate::actions::model::{ActionEvent, LayoutId};
use crate::events::model::EventLog;
use crate::export::camera::{moves::CameraMoveTrack, CameraSim};
use crate::export::cursor::Cursor;
use crate::export::fx::fx_state::{self, FxRenderer};
use crate::export::gpu::compositor::{select_compositor, Compositor};
use crate::export::pipeline::ffio::probe_dims;
use crate::export::pipeline::timeline::build_timeline;
use crate::export::render::bg::build_bg;
use crate::export::render::render_edit::EditState;
use crate::export::scene::background;
use crate::export::{
    settings::Resolution,
    types::{Layout, ZoomConfig, ZoomRegion},
};
use crate::ports::system::SystemPort;
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;
use anyhow::{Context, Result};
mod meta;
pub use meta::{FramePose, RenderMeta};

pub const OUT_FPS: u64 = 60;

pub const OUT_STEP_MS: f32 = 1000.0 / OUT_FPS as f32;

pub struct FrameRenderer {
    settings: Settings,
    cfg: ZoomConfig,
    layout: Layout,
    track: spans::SpanTrack,
    mix_buf: Vec<u8>,
    cam_moves: CameraMoveTrack,
    regions: Vec<ZoomRegion>,
    frame_regions: Vec<ZoomRegion>,
    bg: Vec<u8>,
    compositor: Box<dyn Compositor>,
    fx: Box<dyn FxRenderer>,
    sim: CameraSim,
    spot_sim: fx_state::SpotlightSim,
    cursor: Cursor,
    cprep: Option<crate::export::cursor::cursorset::CursorPrep>,
    captured: Option<crate::export::cursor::captured::CapturedCursors>,
    actions: Vec<ActionEvent>,
    effects: Vec<crate::edit::model::EffectRegion>,
    captions: Vec<crate::edit::captions::Caption>,
    texts: Vec<crate::edit::text::TextItem>,
    has_webcam: bool,
    os_cursor_in_video: bool,
    sw: u32,
    sh: u32,
    events_ms: u64,
    video_start: u64,
    map: crate::export::remap::TimeMap,
    clip_mix: clipmix::ClipMixTrack,
    full_dur_ms: u32,
    grade: Option<crate::export::grade::GradeParams>,
}

impl FrameRenderer {
    pub fn new(
        paths: &ProjectPaths,
        layout: Layout,
        fps: u32,
        resolution: Resolution,
        preview_cap: Option<u32>,
        system: &dyn SystemPort,
    ) -> Result<(Self, RenderMeta)> {
        let log = EventLog::load(&paths.events()).context("load events.json")?;
        let (sw, sh) = probe_dims(&paths.video())?;
        let (sw, sh, screen_crop) = meta::even_screen(sw, sh);
        let seed = crate::edit::seed::load_or_seed(paths);
        let mut layout = layout;
        layout.resolve(seed.aspect, resolution, sw, sh, preview_cap);
        let actions = crate::actions::model::ActionLog::load(&paths.actions())
            .map(|a| a.actions)
            .unwrap_or_default();
        let tl = build_timeline(paths, &log, fps);
        let video_start = tl.frames[0];
        let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
        let full_dur_ms = (video_end - video_start) as u32;
        let es = EditState::load(
            paths,
            &actions,
            &layout,
            sw,
            sh,
            tl.events_ms as i64 - video_start as i64,
            video_start,
            full_dur_ms,
        );
        let screen_bytes = (sw as usize * sh as usize) * 3 / 2;
        let has_webcam = paths.webcam().exists();
        let panels: Vec<_> = [
            LayoutId::Screen,
            LayoutId::Camera,
            LayoutId::Presenter,
            LayoutId::ScreenOnly,
            LayoutId::CameraOnly,
        ]
        .iter()
        .map(|&id| {
            crate::settings::appearance::overlay_for(
                es.settings.appearance.for_id(id),
                layout.out_w,
                layout.out_h,
                true,
            )
        })
        .collect();
        let wc_src = has_webcam
            .then(|| probe_dims(&paths.webcam()).ok())
            .flatten();
        let (webcam_w, webcam_h) = meta::webcam_box(&panels, wc_src, 1440);
        let bg = build_bg(paths, &es.settings.background, layout.out_w, layout.out_h);
        let compositor = select_compositor(&layout);
        compositor.set_bg_dynamic(
            background::video_source(&es.settings.background, &paths.folder).is_some(),
        );
        let fx = fx_state::select_fx(layout.out_w, layout.out_h);
        let (sim, spot_sim) = (
            CameraSim::new(layout.out_w, layout.out_h),
            fx_state::SpotlightSim::new(),
        );
        let events_ms = tl.events_ms;
        let audio_offset_ms = es.settings.audio_offset_ms;
        let (out_w, out_h) = (layout.out_w, layout.out_h);
        let dark =
            crate::settings::theme::resolve_dark(es.settings.ui.theme, system.os_prefers_dark());
        let cursor_track = crate::events::track::cursortype::CursorTrack::load(&paths.cursor());
        let os_cur = crate::settings::store::os_cursor_in_video(paths);
        let cprep = crate::export::cursor::cursorset::prep(
            &es.settings.cursor,
            &log.events,
            cursor_track,
            dark,
            os_cur,
        );
        let captured = crate::export::cursor::captured::CapturedCursors::load(paths);
        let mut cursor = Cursor::new(
            log.events,
            log.screen,
            es.settings.cursor.smoothness_at(os_cur),
        );
        cursor.set_idealize(es.settings.cursor.idealize_at(os_cur));
        cursor.set_tilt(es.settings.cursor.tilt_at(os_cur));
        let meta = RenderMeta {
            tl,
            video_start,
            video_end,
            out_w,
            out_h,
            sw,
            sh,
            screen_bytes,
            screen_crop,
            webcam_w,
            webcam_h,
            audio_offset_ms,
            trim: seed.trim,
            mic_volume: es.settings.audio_mic_volume,
            sys_volume: es.settings.audio_sys_volume,
        };
        Ok((
            Self {
                settings: es.settings,
                cfg: es.cfg,
                layout,
                track: es.track,
                mix_buf: Vec::new(),
                cam_moves: es.cam_moves,
                regions: es.regions,
                frame_regions: Vec::new(),
                bg,
                compositor,
                fx,
                sim,
                spot_sim,
                cursor,
                cprep,
                captured,
                actions,
                effects: es.effects,
                captions: es.captions,
                texts: es.texts,
                has_webcam,
                os_cursor_in_video: os_cur,
                sw,
                sh,
                events_ms,
                video_start,
                map: es.map,
                clip_mix: es.clip_mix,
                full_dur_ms,
                grade: es.grade,
            },
            meta,
        ))
    }
}

mod accessors;
mod composite;
mod fx_step;
mod step;

pub mod bg;
pub mod clipmix;
pub mod fromedit;
pub mod render_edit;

pub mod screen_mix;
pub mod spans;
