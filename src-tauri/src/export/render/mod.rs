// Reusable per-frame renderer: owns all per-export setup except decoders and
// the encoder sink, so a single frame can be composited at an arbitrary time T.
// The exporter calls new() once, then step_camera()+composite_at() per frame.
use anyhow::{Context, Result};
use crate::actions::model::{ActionEvent, LayoutId};
use crate::events::model::EventLog;
use crate::export::scene::background;
use crate::export::camera::{moves::CameraMoveTrack, static_cam_pose, CameraSim};
use crate::export::gpu::compositor::{select_compositor, Compositor};
use crate::export::coordmap::{inset_rect, to_panel};
use crate::export::cursor::Cursor;
use crate::export::pipeline::ffio::{decode_image, probe_dims};
use crate::export::fx::fx_state::{self, FxRenderer};
use crate::export::scene::layout::LayoutTrack;
use crate::export::render::render_edit::EditState;
use crate::export::scene::Scene;
use crate::export::pipeline::timeline::{build_timeline, Timeline};
use crate::export::types::{Background, Camera, FramePoint, Layout, ZoomConfig, ZoomRegion};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

const BG_MESH: &[u8] = include_bytes!("../../../assets/backgrounds/bg.jpg");

/// Constant output frame rate.
pub const OUT_FPS: u64 = 60;

/// What the export loop needs to set up its decoders and drive the frame loop.
pub struct RenderMeta {
    pub tl: Timeline, pub video_start: u64, pub video_end: u64, pub out_w: u32, pub out_h: u32,
    pub sw: u32, pub sh: u32, pub screen_bytes: usize, pub webcam_size: u32, pub audio_offset_ms: i32,
}

/// Camera + scene resolved for one output frame; returned by `step_camera`.
pub struct FramePose { pub ev_t: u32, pub scene: Scene, pub cur: FramePoint, pub cam: Camera }

/// Owns all per-export setup except decoders and the encoder sink. `step_camera` is cheap
/// (math only); `composite_at` runs the full compositor + FX + cursor stack and returns BGRA.
pub struct FrameRenderer {
    settings: Settings,
    cfg: ZoomConfig,
    layout: Layout,
    track: LayoutTrack,
    cam_moves: CameraMoveTrack,
    regions: Vec<ZoomRegion>,
    bg: Vec<u8>,
    compositor: Box<dyn Compositor>,
    fx: Box<dyn FxRenderer>,
    sim: CameraSim,
    spot_sim: fx_state::SpotlightSim,
    cursor: Cursor, // single owner of the event log (also the FX event source)
    cprep: Option<crate::export::cursor::cursorset::CursorPrep>,
    actions: Vec<ActionEvent>,
    effects: Vec<crate::edit::model::EffectRegion>,
    sw: u32,
    sh: u32,
    events_ms: u64,
    video_start: u64, // frame-0 capture time; zoom pills are output-time (t - video_start)
}

impl FrameRenderer {
    /// Load doc/settings/regions/compositor/sim from `paths`; return a `RenderMeta` the exporter
    /// uses to spawn decoders and drive the frame loop. `fps` is the capture-rate fallback.
    pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32) -> Result<(Self, RenderMeta)> {
        let log = EventLog::load(&paths.events()).context("load events.json")?;
        let (sw, sh) = probe_dims(&paths.video())?;
        let actions = crate::actions::model::ActionLog::load(&paths.actions())
            .map(|a| a.actions).unwrap_or_default();
        // Edit-derived state (zoom/layout/regions/effects) - the slice reload_edit refreshes.
        let es = EditState::load(paths, &actions, &layout, sw, sh);
        let tl = build_timeline(paths, &log, fps);
        let video_start = tl.frames[0];
        let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
        let screen_bytes = (sw * sh * 4) as usize;
        let max_cam = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter,
                       LayoutId::ScreenOnly, LayoutId::CameraOnly]
            .iter().map(|&id| crate::settings::appearance::overlay_for(
                es.settings.appearance.for_id(id), layout.out_w, layout.out_h, true).size_px)
            .max().unwrap_or(420);
        let webcam_size = max_cam.min(1440).max(1);
        let bg = decode_image(BG_MESH, layout.out_w, layout.out_h)
            .unwrap_or_else(|_| background::render(&Background::default(), layout.out_w, layout.out_h));
        let compositor = select_compositor(&layout);
        let fx = fx_state::select_fx(layout.out_w, layout.out_h);
        let (sim, spot_sim) = (CameraSim::new(layout.out_w, layout.out_h), fx_state::SpotlightSim::new());
        let events_ms = tl.events_ms;
        let audio_offset_ms = es.settings.audio_offset_ms;
        let (out_w, out_h) = (layout.out_w, layout.out_h);
        // Cursor prep is edit-independent for the warm preview (only composite_at uses it) but
        // dominates a build, so it lives here - NOT in EditState/reload_edit.
        let dark = crate::win::theme::resolve_dark(es.settings.ui.theme);
        let cursor_track = crate::events::track::cursortype::CursorTrack::load(&paths.cursor());
        let cprep = crate::export::cursor::cursorset::prep(&es.settings.cursor, &log.events, cursor_track, dark);
        let cursor = Cursor::new(log.events, log.screen); // moves the log in after cprep borrowed it
        let meta = RenderMeta { tl, video_start, video_end, out_w, out_h, sw, sh, screen_bytes, webcam_size, audio_offset_ms };
        Ok((Self { settings: es.settings, cfg: es.cfg, layout, track: es.track, cam_moves: es.cam_moves, regions: es.regions,
            bg, compositor, fx, sim, spot_sim, cursor, cprep, actions, effects: es.effects, sw, sh, events_ms, video_start }, meta))
    }

    /// Refresh only the `edit.json`-derived state in place (zoom/layout/regions/effects/cam_moves),
    /// WITHOUT recreating the GPU compositor, FX, background, or re-probing dims (all edit-
    /// independent). `with_warm` calls this instead of a full `new()` so edits stay snappy.
    pub fn reload_edit(&mut self, paths: &ProjectPaths) {
        let es = EditState::load(paths, &self.actions, &self.layout, self.sw, self.sh);
        self.settings = es.settings; self.cfg = es.cfg; self.track = es.track;
        self.regions = es.regions; self.effects = es.effects; self.cam_moves = es.cam_moves;
    }

    /// Rewind the forward-only camera + cursor state so this (cached) renderer can be
    /// reused to preview an arbitrary T by fast-forwarding `step_camera` from the start.
    pub fn reset_camera(&mut self) {
        (self.sim, self.spot_sim) = (CameraSim::new(self.layout.out_w, self.layout.out_h), fx_state::SpotlightSim::new());
        self.cursor.reset();
    }

    /// Advance the camera sim to output time `t` (ms) and return the full pose. Cheap: math
    /// only, no decode. `t` MUST be non-decreasing across calls - the cursor sample index and
    /// the camera low-pass only move forward; use a fresh renderer to preview an arbitrary T.
    pub fn step_camera(&mut self, t: u64) -> FramePose {
        let ev_t = (t.saturating_sub(self.events_ms)) as u32;
        let smooth = self.cursor.at(ev_t);
        let mut scene = self.track.scene_at(ev_t);
        let cur = to_panel(smooth, self.sw, self.sh, scene.screen.rect);
        // Zoom pills live in OUTPUT time (t - video_start), not event time; cursor/layout above stay event-time.
        let out_t = t.saturating_sub(self.video_start) as u32;
        let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
        let sp = Some(static_cam_pose(scene.camera.rect, ow, oh)); // implicit t=0 keyframe: the pre-override static pose
        if let Some(p) = self.cam_moves.sample(out_t, sp) {
            scene.camera = crate::export::scene::override_camera(scene.camera, p, ow, oh);
        }
        let mut cam = self.sim.step(out_t, cur, &self.regions, &self.cfg);
        if scene.screen.alpha < 0.5 {
            cam = Camera { cx: self.layout.out_w as f32 / 2.0, cy: self.layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        if self.settings.zoom.camera_shrink && scene.camera.rect.w < scene.screen.rect.w {
            scene.camera = crate::export::scene::shrink_camera(
                scene.camera, cam.scale, self.cfg.target_scale, self.settings.zoom.camera_shrink_min);
        }
        FramePose { ev_t, scene, cur, cam }
    }

    /// Composite one frame at the given pose into `out` (BGRA, `out_w * out_h * 4` bytes).
    pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8],
                        webcam: Option<(&[u8], u32, u32)>, out: &mut Vec<u8>) {
        let (ow, oh) = (self.layout.out_w, self.layout.out_h);
        self.compositor.composite_into(screen, self.sw, self.sh, webcam,
            pose.cam, &self.bg, &self.layout, &pose.scene, out);
        fx_state::render(&*self.fx, out, ow, oh, &self.settings.clickfx,
            self.cursor.events(), &self.actions, &self.effects, &pose.scene, pose.cam, pose.cur,
            self.sw, self.sh, pose.ev_t, &self.settings.hotkeys, &mut self.spot_sim);
        if let Some(cp) = &mut self.cprep {
            crate::export::cursor::cursorset::draw(cp, out, ow, oh, pose.cur, pose.cam,
                &pose.scene.screen, inset_rect(self.sw, self.sh, &self.layout).2 as f32,
                pose.ev_t, &self.settings.cursor);
        }
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

    /// This preset's `Scene`, resolved from the renderer's own appearance + dims (the same
    /// resolve `LayoutTrack` performs) - lets `preview_layouts` cross-fade between presets.
    pub fn resolve_layout(&self, id: crate::actions::model::LayoutId) -> crate::export::scene::Scene {
        let (ma, ow, oh) = (self.settings.appearance.for_id(id), self.layout.out_w, self.layout.out_h);
        crate::export::scene::resolve(id, &crate::settings::appearance::layout_for(ma, ow, oh),
            &crate::settings::appearance::overlay_for(ma, ow, oh, true), self.sw, self.sh)
    }
}
pub mod render_edit;
pub mod fromedit;
