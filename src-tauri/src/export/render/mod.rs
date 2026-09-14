// Reusable per-frame renderer: owns all per-export setup except decoders and the encoder sink, so
// a single frame composites at an arbitrary time T. The exporter calls new() once, then step_camera()+composite_at() per frame.
use anyhow::{Context, Result};
use crate::actions::model::{ActionEvent, LayoutId};
use crate::events::model::EventLog;
use crate::export::scene::background;
use crate::export::render::bg::build_bg;
use crate::export::camera::{moves::CameraMoveTrack, CameraSim};
use crate::export::gpu::compositor::{select_compositor, Compositor};
use crate::export::coordmap::inset_rect;
use crate::export::cursor::Cursor;
use crate::export::pipeline::ffio::probe_dims;
use crate::export::fx::fx_state::{self, FxRenderer};
use crate::export::render::render_edit::EditState;
use crate::export::pipeline::timeline::build_timeline;
use crate::export::{settings::Resolution, types::{Layout, ZoomConfig, ZoomRegion}};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;
mod meta;
pub use meta::{FramePose, RenderMeta};

pub const OUT_FPS: u64 = 60; // constant output frame rate
/// `OUT_FPS`'s exact frame period in ms - `step_camera`'s `dt_ms`, never the integer `1000 / 60`.
pub const OUT_STEP_MS: f32 = 1000.0 / OUT_FPS as f32;

/// Owns all per-export setup except decoders and the encoder sink. `step_camera` is cheap (math only); `composite_at` runs the full compositor + FX + cursor stack and returns BGRA.
pub struct FrameRenderer {
    settings: Settings,
    cfg: ZoomConfig,
    layout: Layout,
    // Sampled at OUT_T: one `LayoutTrack` per SOURCE SPAN (segments from `doc.layout`), so a
    // mid-take display switch re-shapes the screen panel and eases into it (`spans.rs`).
    track: spans::SpanTrack,
    /// Scratch nv12 frame for a display switch's cross-dissolve - only ever touched inside the
    /// 350 ms window (`screen_mix`), where it holds the blended screen the compositor draws.
    mix_buf: Vec<u8>,
    cam_moves: CameraMoveTrack,
    /// Zoom regions with RAW canvas-space anchors (`fromedit::regions_from_doc`), and the scratch
    /// buffer `step_camera` re-anchors them into each frame's own screen panel through.
    regions: Vec<ZoomRegion>,
    frame_regions: Vec<ZoomRegion>,
    bg: Vec<u8>,
    compositor: Box<dyn Compositor>,
    fx: Box<dyn FxRenderer>,
    sim: CameraSim,
    spot_sim: fx_state::SpotlightSim,
    cursor: Cursor, // single owner of the event log (also the FX event source)
    cprep: Option<crate::export::cursor::cursorset::CursorPrep>,
    /// The recording's captured OS-cursor layer, decoded once. `Some` = the real cursor can be
    /// composited, which is what the LIVE `System` style draws instead of the synthetic arrow.
    captured: Option<crate::export::cursor::captured::CapturedCursors>,
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
    map: crate::export::remap::TimeMap, // the clip-to-output clock map every walk and the mux read
    full_dur_ms: u32,
}

impl FrameRenderer {
    /// Load doc/settings/regions/compositor/sim from `paths`; return a `RenderMeta` the exporter uses to
    /// spawn decoders and drive the frame loop. `fps` is the capture-rate fallback; `preview_cap` (`Some(long_edge)`) downscales the resolved aspect frame for a cheap preview (`Layout::resolve`).
    pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32, resolution: Resolution, preview_cap: Option<u32>) -> Result<(Self, RenderMeta)> {
        let log = EventLog::load(&paths.events()).context("load events.json")?;
        let (sw, sh) = probe_dims(&paths.video())?;
        let (sw, sh, screen_crop) = meta::even_screen(sw, sh); // the decoders crop to this (`ffio_decoder`)
        let seed = crate::edit::seed::load_or_seed(paths);
        let mut layout = layout;
        layout.resolve(seed.aspect, resolution, sw, sh, preview_cap);
        let actions = crate::actions::model::ActionLog::load(&paths.actions()).map(|a| a.actions).unwrap_or_default();
        let tl = build_timeline(paths, &log, fps); // before EditState: it needs the event->output shift
        let video_start = tl.frames[0];
        let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
        let full_dur_ms = (video_end - video_start) as u32;
        let es = EditState::load(paths, &actions, &layout, sw, sh, tl.events_ms as i64 - video_start as i64, video_start, full_dur_ms);
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
        let bg = build_bg(paths, &es.settings.background, layout.out_w, layout.out_h);
        let compositor = select_compositor(&layout);
        // A VIDEO background replaces `bg` every frame, so the compositor must stop deciding
        // whether to re-upload from the buffer's content key alone (see `gpu_compositor_tex`).
        compositor.set_bg_dynamic(background::video_source(&es.settings.background, &paths.folder).is_some());
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
        let captured = crate::export::cursor::captured::CapturedCursors::load(paths);
        let mut cursor = Cursor::new(log.events, log.screen, es.settings.cursor.smoothness_at(os_cur)); // moves the log in after cprep borrowed it
        cursor.set_idealize(es.settings.cursor.idealize_at(os_cur));
        cursor.set_tilt(es.settings.cursor.tilt_at(os_cur));
        let meta = RenderMeta { tl, video_start, video_end, out_w, out_h, sw, sh, screen_bytes, screen_crop, webcam_w, webcam_h, audio_offset_ms,
            trim: seed.trim, mic_volume: es.settings.audio_mic_volume, sys_volume: es.settings.audio_sys_volume };
        Ok((Self { settings: es.settings, cfg: es.cfg, layout, track: es.track, mix_buf: Vec::new(),
            cam_moves: es.cam_moves, regions: es.regions, frame_regions: Vec::new(),
            bg, compositor, fx, sim, spot_sim, cursor, cprep, captured, actions, effects: es.effects, has_webcam,
            os_cursor_in_video: os_cur, sw, sh, events_ms, video_start, map: es.map, full_dur_ms }, meta))
    }

    /// Composite one frame. `prev` is the screen frame the caller latched before the display
    /// switch this frame is inside (`FramePose::hold`/`mix`), or `None` outside a switch - it is
    /// blended into the current frame's span rect BEFORE compositing (`screen_mix`), which is what
    /// keeps the cross-dissolve on one code path for the CPU compositor, the GPU one, the preview
    /// and the export alike.
    pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8], prev: Option<&[u8]>,
                        webcam: Option<(&[u8], u32, u32)>, out: &mut Vec<u8>) {
        let (ow, oh) = (self.layout.out_w, self.layout.out_h);
        let screen = match (pose.mix, prev) {
            (Some(m), Some(p)) => {
                screen_mix::blend_into(&mut self.mix_buf, screen, p, self.sw, self.sh,
                    m.prev_src, pose.scene.src, m.alpha);
                &self.mix_buf[..]
            }
            _ => screen,
        };
        self.compositor.composite_into(screen, self.sw, self.sh, webcam,
            pose.cam, &self.bg, &self.layout, &pose.scene, out);
        // The real captured cursor wins whenever the live style is System and this recording has
        // a layer; otherwise the synthetic stack (Enhanced, or the plain arrow on a pre-layer
        // recording) runs exactly as before. Both take the same placement arguments.
        let inset_w = inset_rect(self.sw, self.sh, &self.layout).2 as f32;
        let sc = &pose.scene.screen;
        let captured = crate::export::cursor::captured::draws_captured(self.settings.cursor.style, self.captured.is_some());
        // The glass cursor material is placed BEFORE the FX pass (which renders it) even though
        // the sprite it belongs to is blitted AFTER - see `fx_lensbuild::lenses_at`. A real
        // captured OS cursor never gets one: it is the cursor that was actually on screen.
        let lens = (!captured).then(|| self.lenses(pose, ow, oh, sc, inset_w)).flatten();
        fx_state::render(&*self.fx, out, ow, oh, &self.settings.clickfx,
            self.cursor.events(), &self.actions, &self.effects, &pose.scene, pose.cam, pose.cur, &self.cursor.screen(), self.has_webcam,
            pose.out_t, pose.ev_t, &self.settings.hotkeys, &mut self.spot_sim, lens);
        if captured {
            // The SPAN's width, not the canvas's: the captured bitmap is in source pixels and the
            // panel shows `src.w` of them, so that is what one source pixel measures on screen.
            let src_w = pose.scene.src.w.max(1.0) as u32;
            if let Some(cc) = &self.captured { cc.draw(out, ow, oh, pose.cur, pose.cam, sc, inset_w, src_w, pose.ev_t); }
        } else if let Some(cp) = &mut self.cprep {
            crate::export::cursor::cursorset::draw(cp, out, ow, oh, pose.cur, pose.cam,
                sc, inset_w, pose.ev_t, pose.out_t, &self.settings.cursor, self.os_cursor_in_video,
                self.cursor.tilt_deg());
        }
    }

    /// The glass lens + cursor back for this frame, or `None` when neither is asked for. One line
    /// of plumbing kept out of `composite_at` so that function stays readable; everything it
    /// decides lives in `fx_lensbuild`.
    fn lenses(&self, pose: &FramePose, ow: u32, oh: u32, sc: &crate::export::scene::Panel,
              inset_w: f32) -> Option<crate::export::fx::fx_lens::Lenses> {
        use crate::export::fx::fx_lensbuild::{lenses_at, wants_lens, LensFrame};
        let c = &self.settings.cursor;
        let plain_os = c.plain_os(self.os_cursor_in_video);
        if !wants_lens(self.cprep.as_ref(), c, plain_os) { return None; }
        let info = self.cursor.screen();
        let f = LensFrame { cur: pose.cur, cam: pose.cam, ow, oh, screen: sc, inset_w,
            info: &info, src: pose.scene.src, ev_t: pose.ev_t, out_t: pose.out_t,
            tilt_deg: self.cursor.tilt_deg() };
        lenses_at(self.cprep.as_ref()?, c, f, plain_os)
    }
}
// Small read-only accessors used only by preview commands outside the renderer (never by the
// export loop) - split out purely for size; see `accessors.rs`.
mod accessors;
mod step;
// The background: building the static buffer, refreshing it on an edit, swapping in a decoded
// video frame. Also split out for size; see `bg.rs`.
pub mod bg;
pub mod render_edit;
pub mod fromedit;
// The take's source spans (a mid-take display switch) and the cross-dissolve at a switch.
pub mod spans;
pub mod screen_mix;
