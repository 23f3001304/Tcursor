// Reusable per-frame renderer: owns all per-export setup except decoders and the encoder sink, so
// a single frame composites at an arbitrary time T. The exporter calls new() once, then step_camera()+composite_at() per frame.
use anyhow::{Context, Result};
use crate::actions::model::{ActionEvent, LayoutId};
use crate::events::model::EventLog;
use crate::export::scene::background;
use crate::export::camera::{moves::CameraMoveTrack, static_cam_pose, CameraSim};
use crate::export::gpu::compositor::{select_compositor, Compositor};
use crate::export::coordmap::{inset_rect, to_panel};
use crate::export::cursor::Cursor;
use crate::export::pipeline::ffio::probe_dims;
use crate::export::fx::fx_state::{self, FxRenderer};
use crate::export::scene::layout::LayoutTrack;
use crate::export::render::render_edit::EditState;
use crate::export::pipeline::timeline::build_timeline;
use crate::export::{settings::Resolution, types::{Camera, Layout, ZoomConfig, ZoomRegion}};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;
mod meta;
pub use meta::{FramePose, RenderMeta};

const BG_MESH: &[u8] = include_bytes!("../../../assets/backgrounds/bg.jpg");

pub const OUT_FPS: u64 = 60; // constant output frame rate

/// Owns all per-export setup except decoders and the encoder sink. `step_camera` is cheap (math only); `composite_at` runs the full compositor + FX + cursor stack and returns BGRA.
pub struct FrameRenderer {
    settings: Settings,
    cfg: ZoomConfig,
    layout: Layout,
    track: LayoutTrack, // sampled at OUT_T (its segments come from `doc.layout`)
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
    has_webcam: bool, // does `webcam.mp4` exist on disk - gates the spotlight camera-exclusion hole
    // Record-time truth (from the immutable `settings.json` snapshot, NOT the editable doc): does
    // the video already contain a baked OS cursor. Gates the plain-OS synthetic cursor fallback.
    os_cursor_in_video: bool,
    sw: u32, sh: u32,
    // The two clock origins: `t - events_ms` is event time (raw streams), `t - video_start` is
    // output time (every doc region). See `step_camera`.
    events_ms: u64, video_start: u64,
}

impl FrameRenderer {
    /// Load doc/settings/regions/compositor/sim from `paths`; return a `RenderMeta` the exporter uses to
    /// spawn decoders and drive the frame loop. `fps` is the capture-rate fallback; `preview_cap` (`Some(long_edge)`) downscales the resolved aspect frame for a cheap preview (`Layout::resolve`).
    pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32, resolution: Resolution, preview_cap: Option<u32>) -> Result<(Self, RenderMeta)> {
        let log = EventLog::load(&paths.events()).context("load events.json")?;
        let (sw, sh) = probe_dims(&paths.video())?;
        let seed = crate::edit::seed::load_or_seed(paths);
        let mut layout = layout;
        layout.resolve(seed.aspect, resolution, sw, sh, preview_cap);
        let actions = crate::actions::model::ActionLog::load(&paths.actions()).map(|a| a.actions).unwrap_or_default();
        let tl = build_timeline(paths, &log, fps); // before EditState: it needs the event->output shift
        let video_start = tl.frames[0];
        let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
        let es = EditState::load(paths, &actions, &layout, sw, sh, tl.events_ms as i64 - video_start as i64);
        let screen_bytes = (sw as usize * sh as usize) * 3 / 2; // nv12: Y plane + half-res interleaved UV
        // ONE webcam decode box for the whole export, at the SOURCE's own aspect and big enough
        // for every mode's panel; each panel cover-crops it to its own aspect at composite time
        // (`webcam_box`). Picking any single PANEL's aspect here (the old `max_by_key`) stretched
        // the webcam in every layout that disagreed with the winner - and one always did.
        let has_webcam = paths.webcam().exists();
        let panels: Vec<_> = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter, LayoutId::ScreenOnly, LayoutId::CameraOnly]
            .iter().map(|&id| crate::settings::appearance::overlay_for(
                es.settings.appearance.for_id(id), layout.out_w, layout.out_h, true)).collect();
        let wc_src = has_webcam.then(|| probe_dims(&paths.webcam()).ok()).flatten();
        let (webcam_w, webcam_h) = meta::webcam_box(&panels, wc_src, 1440);
        let bg = background::build(&es.settings.background, BG_MESH, layout.out_w, layout.out_h);
        let compositor = select_compositor(&layout);
        let fx = fx_state::select_fx(layout.out_w, layout.out_h);
        let (sim, spot_sim) = (CameraSim::new(layout.out_w, layout.out_h), fx_state::SpotlightSim::new());
        let events_ms = tl.events_ms;
        let audio_offset_ms = es.settings.audio_offset_ms;
        let (out_w, out_h) = (layout.out_w, layout.out_h);
        // Cursor prep is edit-independent for the warm preview (only composite_at uses it) but dominates a build, so it lives here - NOT in EditState/reload_edit.
        let dark = crate::win::theme::resolve_dark(es.settings.ui.theme);
        let cursor_track = crate::events::track::cursortype::CursorTrack::load(&paths.cursor());
        let os_cur = crate::settings::store::os_cursor_in_video(paths);
        let cprep = crate::export::cursor::cursorset::prep(&es.settings.cursor, &log.events, cursor_track, dark, os_cur);
        let mut cursor = Cursor::new(log.events, log.screen, es.settings.cursor.follow_alpha_at(os_cur)); // moves the log in after cprep borrowed it
        cursor.set_idealize(es.settings.cursor.idealize_at(os_cur));
        let meta = RenderMeta { tl, video_start, video_end, out_w, out_h, sw, sh, screen_bytes, webcam_w, webcam_h, audio_offset_ms,
            trim: seed.trim, mic_volume: es.settings.audio_mic_volume, sys_volume: es.settings.audio_sys_volume };
        Ok((Self { settings: es.settings, cfg: es.cfg, layout, track: es.track, cam_moves: es.cam_moves, regions: es.regions,
            bg, compositor, fx, sim, spot_sim, cursor, cprep, actions, effects: es.effects, has_webcam,
            os_cursor_in_video: os_cur, sw, sh, events_ms, video_start }, meta))
    }

    /// Refresh the `edit.json`-derived state in place (zoom/layout/regions/effects/cam_moves), WITHOUT
    /// recreating the GPU compositor or re-probing dims. Also rebuilds `bg`, but ONLY when background
    /// settings changed (the `Mesh` decode shells out to ffmpeg - every OTHER edit must stay cheap).
    pub fn reload_edit(&mut self, paths: &ProjectPaths) {
        let es = EditState::load(paths, &self.actions, &self.layout, self.sw, self.sh,
            self.events_ms as i64 - self.video_start as i64);
        if es.settings.background != self.settings.background {
            self.bg = background::build(&es.settings.background, BG_MESH, self.layout.out_w, self.layout.out_h);
        }
        // Live-apply Smoothness/idealize; the `_at` variants also flip the path to RAW the moment
        // the style is switched to System on a recording with no baked OS cursor.
        self.cursor.set_a(es.settings.cursor.follow_alpha_at(self.os_cursor_in_video));
        self.cursor.set_idealize(es.settings.cursor.idealize_at(self.os_cursor_in_video));
        self.settings = es.settings; self.cfg = es.cfg; self.track = es.track;
        self.regions = es.regions; self.effects = es.effects; self.cam_moves = es.cam_moves;
    }

    /// Rewind the forward-only camera + cursor state so this (cached) renderer can be
    /// reused to preview an arbitrary T by fast-forwarding `step_camera` from the start.
    pub fn reset_camera(&mut self) {
        (self.sim, self.spot_sim) = (CameraSim::new(self.layout.out_w, self.layout.out_h), fx_state::SpotlightSim::new());
        self.cursor.reset();
        // The motion-trail history is forward-only state too: it gains one entry per
        // `composite_at`, and a preview scrub composites exactly ONE frame per call - so without
        // this the trail accumulates the last six SCRUB TARGETS and draws ghost cursors at those
        // unrelated points (default `motion_blur` 0.35 makes them visible).
        if let Some(cp) = &mut self.cprep { cp.recent.clear(); }
    }

    /// Advance the camera sim to capture-clock time `t` (ms) and return the full pose. Cheap: math only, no
    /// decode. `t` MUST be non-decreasing across calls - cursor sample index + camera low-pass only move forward; use a fresh renderer to preview an arbitrary T.
    pub fn step_camera(&mut self, t: u64) -> FramePose {
        // Two clocks: `ev_t` indexes the raw event streams (cursor/mouse/actions), `out_t` is the
        // timeline EVERY EditDoc region list lives on (zooms, layout, effects, camera_moves).
        // Sample each consumer with its own base - never mix them.
        let (ev_t, out_t) = (t.saturating_sub(self.events_ms) as u32, t.saturating_sub(self.video_start) as u32);
        let smooth = self.cursor.at(ev_t);
        let mut scene = self.track.scene_at(out_t);
        let cur = to_panel(smooth, self.sw, self.sh, scene.screen.rect);
        let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
        // The LIVE layout-resolved PiP pose for this frame, read before any override: the keyframe
        // track eases out of it entering its span and back INTO it leaving (re-read every frame, so
        // an exit blend tracks a layout transition that is still moving). Not a one-off static pose.
        let live = Some(static_cam_pose(scene.camera.rect, ow, oh));
        // A pose carries height only, so the override restores width from the STATIC panel's own
        // aspect - otherwise one keyframe silently squares a Wide (16:9) panel for the whole clip.
        let cam_aspect = scene.camera.rect.w / scene.camera.rect.h.max(0.001);
        // Keyframes win WHILE THEY OWN THE FRAME: inside `cam_moves.span()` the smart zoom action is
        // skipped entirely (the shrink used to run ON TOP of an override, silently scaling a
        // hand-keyframed camera during zooms). Outside the span `sample` is None and layout owns it.
        let keyframed = match self.cam_moves.sample(out_t, live) {
            Some(p) => { scene.camera = crate::export::scene::override_camera(scene.camera, p, ow, oh, cam_aspect); true }
            None => false,
        };
        let mut cam = self.sim.step(out_t, cur, &self.regions, &self.cfg);
        if scene.screen.alpha < 0.5 {
            cam = Camera { cx: self.layout.out_w as f32 / 2.0, cy: self.layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        if !keyframed && scene.camera.rect.w < scene.screen.rect.w {
            let (action, target_scale) = crate::export::scene::cam_action_at(&self.regions, &self.settings.zoom, out_t);
            scene.camera = crate::export::scene::apply_cam_zoom_action(
                scene.camera, action, cam.scale, target_scale);
        }
        FramePose { ev_t, out_t, scene, cur, cam }
    }

    /// Composite one frame at the given pose into `out` (BGRA, `out_w * out_h * 4` bytes).
    pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8],
                        webcam: Option<(&[u8], u32, u32)>, out: &mut Vec<u8>) {
        let (ow, oh) = (self.layout.out_w, self.layout.out_h);
        self.compositor.composite_into(screen, self.sw, self.sh, webcam,
            pose.cam, &self.bg, &self.layout, &pose.scene, out);
        fx_state::render(&*self.fx, out, ow, oh, &self.settings.clickfx,
            self.cursor.events(), &self.actions, &self.effects, &pose.scene, pose.cam, pose.cur, &self.cursor.screen(), self.has_webcam,
            self.sw, self.sh, pose.out_t, pose.ev_t, &self.settings.hotkeys, &mut self.spot_sim);
        if let Some(cp) = &mut self.cprep {
            crate::export::cursor::cursorset::draw(cp, out, ow, oh, pose.cur, pose.cam,
                &pose.scene.screen, inset_rect(self.sw, self.sh, &self.layout).2 as f32,
                pose.ev_t, &self.settings.cursor, self.os_cursor_in_video);
        }
    }
}
// Small read-only accessors used only by preview commands outside the renderer (never by the
// export loop) - split out purely for size; see `accessors.rs`.
mod accessors;
pub mod render_edit;
pub mod fromedit;
