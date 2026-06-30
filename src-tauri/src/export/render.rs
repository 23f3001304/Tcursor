// Reusable per-frame renderer: owns all per-export setup except decoders and
// the encoder sink, so a single frame can be composited at an arbitrary time T.
// The exporter calls new() once, then step_camera()+composite_at() per frame.
use anyhow::{Context, Result};
use crate::actions::model::{ActionEvent, LayoutId};
use crate::events::model::EventLog;
use crate::export::background;
use crate::export::camera::CameraSim;
use crate::export::compositor::{Compositor, CpuCompositor};
use crate::export::coordmap::{inset_rect, to_panel};
use crate::export::cursor::Cursor;
use crate::export::ffio::{decode_image, probe_dims};
use crate::export::fx_state::{self, FxRenderer};
use crate::export::layout::LayoutTrack;
use crate::export::scene::Scene;
use crate::export::timeline::{build_timeline, Timeline};
use crate::export::types::{Background, Camera, FramePoint, Layout, ZoomConfig, ZoomRegion};
use crate::export::{gpu, gpu_compositor::GpuCompositor};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

const BG_MESH: &[u8] = include_bytes!("../../assets/backgrounds/bg.jpg");

/// Constant output frame rate.
pub const OUT_FPS: u64 = 60;

/// What the export loop needs to set up its decoders and drive the frame loop.
pub struct RenderMeta {
    pub tl: Timeline,
    pub video_start: u64,
    pub video_end: u64,
    pub out_w: u32,
    pub out_h: u32,
    pub sw: u32,
    pub sh: u32,
    pub screen_bytes: usize,
    pub webcam_size: u32,
    pub audio_offset_ms: i32,
}

/// Camera + scene resolved for one output frame; returned by `step_camera`.
pub struct FramePose {
    pub ev_t: u32,
    pub scene: Scene,
    pub cur: FramePoint,
    pub cam: Camera,
}

/// Owns all per-export setup except decoders and the encoder sink.
/// `step_camera` is cheap (math only); `composite_at` runs the full
/// compositor + FX + cursor stack and returns a BGRA buffer.
pub struct FrameRenderer {
    settings: Settings,
    cfg: ZoomConfig,
    layout: Layout,
    track: LayoutTrack,
    regions: Vec<ZoomRegion>,
    bg: Vec<u8>,
    compositor: Box<dyn Compositor>,
    fx: Box<dyn FxRenderer>,
    sim: CameraSim,
    cursor: Cursor, // single owner of the event log (also the FX event source)
    cprep: Option<crate::export::cursorset::CursorPrep>,
    actions: Vec<ActionEvent>,
    effects: Vec<crate::edit::model::EffectRegion>,
    sw: u32,
    sh: u32,
    events_ms: u64,
}

impl FrameRenderer {
    /// Load doc/settings/regions/compositor/sim from `paths`; return a `RenderMeta`
    /// the exporter uses to spawn decoders and drive the frame loop.
    /// `fps` is the capture frame rate (last-resort fallback in `build_timeline`).
    pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32) -> Result<(Self, RenderMeta)> {
        let log = EventLog::load(&paths.events()).context("load events.json")?;
        let doc = crate::edit::seed::load_or_seed(paths);
        let settings = doc.settings.clone();
        let cfg = settings.zoom.to_zoom_config();
        let (sw, sh) = probe_dims(&paths.video())?;
        const TRANSITION_MS: u32 = 350;
        let actions = crate::actions::model::ActionLog::load(&paths.actions())
            .map(|a| a.actions).unwrap_or_default();
        let layout_acts = crate::export::fromedit::layout_segs_from_doc(&doc);
        let track = LayoutTrack::new(
            layout_acts.as_deref().unwrap_or(&actions), &settings.appearance,
            layout.out_w, layout.out_h, sw, sh, TRANSITION_MS);
        let regions = crate::export::layout::anchor_regions(
            crate::export::fromedit::regions_from_doc(&doc, sw, sh), &track, sw, sh);
        let tl = build_timeline(paths, &log, fps);
        let video_start = tl.frames[0];
        let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
        let screen_bytes = (sw * sh * 4) as usize;
        let max_cam = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter,
                       LayoutId::ScreenOnly, LayoutId::CameraOnly]
            .iter().map(|&id| crate::settings::appearance::overlay_for(
                settings.appearance.for_id(id), layout.out_w, layout.out_h, true).size_px)
            .max().unwrap_or(420);
        let webcam_size = max_cam.min(1440).max(1);
        let bg = decode_image(BG_MESH, layout.out_w, layout.out_h)
            .unwrap_or_else(|_| background::render(&Background::default(), layout.out_w, layout.out_h));
        let compositor = select_compositor(&layout);
        let fx = fx_state::select_fx(layout.out_w, layout.out_h);
        let sim = CameraSim::new(layout.out_w, layout.out_h);
        let cursor_track = crate::events::cursortype::CursorTrack::load(&paths.cursor());
        let dark = crate::win::theme::resolve_dark(settings.ui.theme);
        let cprep = crate::export::cursorset::prep(&settings.cursor, &log.events, cursor_track, dark);
        let events_ms = tl.events_ms;
        let audio_offset_ms = settings.audio_offset_ms;
        let (out_w, out_h) = (layout.out_w, layout.out_h);
        let cursor = Cursor::new(log.events, log.screen); // moves the log in after cprep borrows it
        let meta = RenderMeta { tl, video_start, video_end, out_w, out_h, sw, sh, screen_bytes, webcam_size, audio_offset_ms };
        Ok((Self { settings, cfg, layout, track, regions, bg, compositor, fx, sim,
            cursor, cprep, actions, effects: doc.effects.clone(), sw, sh, events_ms }, meta))
    }

    /// Rewind the forward-only camera + cursor state so this (cached) renderer can be
    /// reused to preview an arbitrary T by fast-forwarding `step_camera` from the start.
    pub fn reset_camera(&mut self) {
        self.sim = CameraSim::new(self.layout.out_w, self.layout.out_h);
        self.cursor.reset();
    }

    /// Advance the camera sim to output time `t` (ms) and return the full pose.
    /// Cheap: math only, no decode. `t` MUST be non-decreasing across calls - the
    /// cursor sample index and the camera low-pass only move forward. To preview an
    /// arbitrary T, use a fresh renderer and fast-forward step_camera from the start.
    pub fn step_camera(&mut self, t: u64) -> FramePose {
        let ev_t = (t.saturating_sub(self.events_ms)) as u32;
        let smooth = self.cursor.at(ev_t);
        let mut scene = self.track.scene_at(ev_t);
        let cur = to_panel(smooth, self.sw, self.sh, scene.screen.rect);
        let mut cam = self.sim.step(ev_t, cur, &self.regions, &self.cfg);
        if scene.screen.alpha < 0.5 {
            cam = Camera { cx: self.layout.out_w as f32 / 2.0,
                cy: self.layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        if self.settings.zoom.camera_shrink && scene.camera.rect.w < scene.screen.rect.w {
            scene.camera = crate::export::scene::shrink_camera(
                scene.camera, cam.scale, self.cfg.target_scale, self.settings.zoom.camera_shrink_min);
        }
        FramePose { ev_t, scene, cur, cam }
    }

    /// Composite one frame at the given pose. Returns BGRA `out_w * out_h * 4` bytes.
    pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8],
                        webcam: Option<(&[u8], u32, u32)>) -> Vec<u8> {
        let (ow, oh) = (self.layout.out_w, self.layout.out_h);
        let mut out = self.compositor.composite(screen, self.sw, self.sh, webcam,
            pose.cam, &self.bg, &self.layout, &pose.scene);
        fx_state::render(&*self.fx, &mut out, ow, oh, &self.settings.clickfx,
            self.cursor.events(), &self.actions, &self.effects, &pose.scene, pose.cam, pose.cur,
            self.sw, self.sh, pose.ev_t, &self.settings.hotkeys);
        if let Some(cp) = &mut self.cprep {
            crate::export::cursorset::draw(cp, &mut out, ow, oh, pose.cur, pose.cam,
                &pose.scene.screen, inset_rect(self.sw, self.sh, &self.layout).2 as f32,
                pose.ev_t, &self.settings.cursor);
        }
        out
    }

    /// The export background buffer (BGRA, `out_w` x `out_h`) - the same mesh/gradient the
    /// compositor draws under the screen. Exposed so the editor preview shows the exact bg.
    pub fn bg(&self) -> &[u8] { &self.bg }

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
}

/// GPU compositor if available, else CPU fallback. Never fails.
pub fn select_compositor(layout: &Layout) -> Box<dyn Compositor> {
    if gpu::gpu_available() {
        if let Some(c) = GpuCompositor::new(layout.out_w, layout.out_h) { return Box::new(c); }
    }
    Box::new(CpuCompositor)
}
