use super::{FramePose, FrameRenderer};
use crate::export::camera::{static_cam_pose, CameraSim};
use crate::export::coordmap::to_panel;
use crate::export::fx::fx_state;
use crate::export::types::Camera;

impl FrameRenderer {
    pub fn reset_camera(&mut self) {
        (self.sim, self.spot_sim) = (
            CameraSim::new(self.layout.out_w, self.layout.out_h),
            fx_state::SpotlightSim::new(),
        );
        self.cursor.reset();
        if let Some(cp) = &mut self.cprep {
            cp.recent.clear();
        }
    }

    pub fn snap_cursor(&mut self) {
        self.cursor.reset();
        if let Some(cp) = &mut self.cprep {
            cp.recent.clear();
        }
    }

    pub fn step_camera(&mut self, t_clip_abs: u64, t_out: u32, dt_ms: f32) -> FramePose {
        let (ev_t, out_t) = (t_clip_abs.saturating_sub(self.events_ms) as u32, t_out);
        let smooth = self.cursor.at(ev_t, dt_ms);
        let fps = (1000.0 / dt_ms.max(0.001)).round().max(1.0) as u64;
        let (mut scene, mix, hold) = self.track.frame_at(out_t, fps);
        let cur = to_panel(smooth, scene.src, scene.screen.rect);
        let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
        let live = Some(static_cam_pose(&scene.camera, ow, oh));
        let cam_aspect = scene.camera.rect.w / scene.camera.rect.h.max(0.001);
        let keyframed = match self.cam_moves.sample(out_t, live) {
            Some(p) => {
                scene.camera =
                    crate::export::scene::override_camera(scene.camera, p, ow, oh, cam_aspect);
                true
            }
            None => false,
        };
        crate::export::scene::layout::anchor_frame(&self.regions, &scene, &mut self.frame_regions);
        let mut cam = self
            .sim
            .step(out_t, dt_ms, cur, &self.frame_regions, &self.cfg);
        if scene.screen.alpha < 0.5 {
            cam = Camera {
                cx: self.layout.out_w as f32 / 2.0,
                cy: self.layout.out_h as f32 / 2.0,
                scale: 1.0,
            };
        }
        if !keyframed && scene.camera.rect.w < scene.screen.rect.w {
            let (action, target_scale) =
                crate::export::scene::cam_action_at(&self.regions, &self.settings.zoom, out_t);
            scene.camera = crate::export::scene::apply_cam_zoom_action(
                scene.camera,
                action,
                cam.scale,
                target_scale,
            );
        }
        let clip_mix = self.clip_mix.at(out_t);
        FramePose {
            ev_t,
            out_t,
            scene,
            cur,
            cam,
            mix,
            hold,
            clip_mix,
        }
    }

    pub fn walk_plan(
        &mut self,
        video_start: u64,
        fps: u64,
        plan: &[u64],
        last_j: usize,
        dt_ms: f32,
        mut f: impl FnMut(&mut FrameRenderer, usize, u64, &FramePose) -> bool,
    ) -> Option<FramePose> {
        let snaps = self.map.plan_boundaries(fps);
        self.clip_mix.resolve(&self.map, fps);
        let ms = |k: u64| k * 1000 / fps;
        for k in 0..plan.first().copied().unwrap_or(0) {
            self.step_camera(video_start + ms(k), 0, dt_ms);
        }
        let mut last = None;
        for (j, &k) in plan.iter().enumerate().take(last_j.saturating_add(1)) {
            if snaps.binary_search(&j).is_ok() {
                self.snap_cursor();
            }
            let pose = self.step_camera(video_start + ms(k), ms(j as u64) as u32, dt_ms);
            let go = f(self, j, k, &pose);
            last = Some(pose);
            if !go {
                break;
            }
        }
        last
    }
}
