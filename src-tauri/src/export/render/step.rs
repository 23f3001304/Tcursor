// The per-frame camera step, moved out of render/mod.rs (at the size cap) and given the two clocks
// the time remap needs. `t_clip_abs` is absolute clip time (`video_start + k * 1000 / fps`) and feeds
// the cursor and the raw event streams; `t_out` is output time in ms and feeds the layout track, the
// regions, the camera moves and the simulation, which all live on the output clock since
// `edit::remap_doc`. `dt_ms` stays the exact output frame period, so the camera runs on the output
// clock and every per-ms filter downstream is untouched by cuts and speed spans.
use super::{FramePose, FrameRenderer};
use crate::export::camera::{static_cam_pose, CameraSim};
use crate::export::coordmap::to_panel;
use crate::export::fx::fx_state;
use crate::export::types::Camera;

impl FrameRenderer {
    pub fn reset_camera(&mut self) {
        (self.sim, self.spot_sim) = (CameraSim::new(self.layout.out_w, self.layout.out_h), fx_state::SpotlightSim::new());
        self.cursor.reset();
        if let Some(cp) = &mut self.cprep { cp.recent.clear(); }
    }

    /// Forget the smoothed cursor so the next step starts on the raw position: at a cut the viewer
    /// never saw the frames the filter would glide across. The camera itself keeps its state.
    pub fn snap_cursor(&mut self) {
        self.cursor.reset();
        if let Some(cp) = &mut self.cprep { cp.recent.clear(); }
    }

    pub fn step_camera(&mut self, t_clip_abs: u64, t_out: u32, dt_ms: f32) -> FramePose {
        // Two clocks: `ev_t` indexes the raw event streams (cursor/mouse/actions) and comes off the
        // clip clock; `out_t` is the timeline EVERY EditDoc region list lives on after `remap_doc`.
        // Sample each consumer with its own base - never mix them.
        let (ev_t, out_t) = (t_clip_abs.saturating_sub(self.events_ms) as u32, t_out);
        let smooth = self.cursor.at(ev_t, dt_ms);
        // `dt_ms` IS the output frame period, so it names the frame rate the caller is producing -
        // which is what decides exactly which frame a display switch dissolves FROM (`hold_tick`).
        let fps = (1000.0 / dt_ms.max(0.001)).round().max(1.0) as u64;
        let (mut scene, mix, hold) = self.track.frame_at(out_t, fps);
        // Through the SPAN's source rect, not the canvas: after a display switch the panel shows
        // only the fitted sub-rect, so a canvas point maps into the panel through that rect.
        let cur = to_panel(smooth, scene.src, scene.screen.rect);
        let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
        // The LIVE layout-resolved PiP pose for this frame, read before any override: the keyframe
        // track eases out of it entering its span and back INTO it leaving (re-read every frame, so
        // an exit blend tracks a layout transition that is still moving). Not a one-off static pose.
        let live = Some(static_cam_pose(&scene.camera, ow, oh));
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
        // A pinned aim lives in canvas coords; put it into THIS frame's panel (through this frame's
        // crop rect), so a layout transition or display switch mid-zoom carries the aim with the
        // content instead of leaving the camera zooming into where the panel used to be.
        crate::export::scene::layout::anchor_frame(&self.regions, &scene, &mut self.frame_regions);
        let mut cam = self.sim.step(out_t, dt_ms, cur, &self.frame_regions, &self.cfg);
        if scene.screen.alpha < 0.5 {
            cam = Camera { cx: self.layout.out_w as f32 / 2.0, cy: self.layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        if !keyframed && scene.camera.rect.w < scene.screen.rect.w {
            let (action, target_scale) = crate::export::scene::cam_action_at(&self.regions, &self.settings.zoom, out_t);
            scene.camera = crate::export::scene::apply_cam_zoom_action(
                scene.camera, action, cam.scale, target_scale);
        }
        FramePose { ev_t, out_t, scene, cur, cam, mix, hold }
    }

    /// The one walk the exporter, the one-shot preview and `camera_track` share, so their cameras
    /// agree: warm up over the recording frames before the first kept one (stepped at output time 0,
    /// so the camera settles into the state output frame 0 needs on the cursor's real path), then
    /// step every output frame of `plan` through `last_j`, snapping the cursor across a cut, calling
    /// `f(renderer, j, k, pose)` after each; `f` returns false to stop. The last pose stepped.
    pub fn walk_plan(&mut self, video_start: u64, fps: u64, plan: &[u64], last_j: usize, dt_ms: f32,
                     mut f: impl FnMut(&mut FrameRenderer, usize, u64, &FramePose) -> bool) -> Option<FramePose> {
        let map = self.map.clone();
        let ms = |k: u64| k * 1000 / fps;
        for k in 0..plan.first().copied().unwrap_or(0) { self.step_camera(video_start + ms(k), 0, dt_ms); }
        let mut last = None;
        for (j, &k) in plan.iter().enumerate().take(last_j.saturating_add(1)) {
            if j > 0 && map.crosses_cut(plan[j - 1], k, fps) { self.snap_cursor(); }
            let pose = self.step_camera(video_start + ms(k), ms(j as u64) as u32, dt_ms);
            let go = f(self, j, k, &pose);
            last = Some(pose);
            if !go { break; }
        }
        last
    }
}
