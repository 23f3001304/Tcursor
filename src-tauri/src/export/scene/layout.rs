use crate::actions::model::LayoutId;
use crate::actions::model::{ActionEvent, ActionKind};
use crate::edit::model::LayoutSeg;
use crate::export::camera::{ease, fit_durations};
use crate::export::coordmap::to_panel;
use crate::export::scene::{resolve, Scene};
use crate::export::types::{Easing, ZoomRegion};
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

pub fn resolve_seg_scene(
    s: &LayoutSeg,
    app: &AppearanceSettings,
    ow: u32,
    oh: u32,
    sw: u32,
    sh: u32,
) -> Scene {
    let id = crate::export::render::fromedit::layout_id_from(&s.layout);
    let ma = app.for_id(id);
    let base = resolve(
        id,
        &layout_for(ma, ow, oh),
        &overlay_for(ma, ow, oh, true),
        sw,
        sh,
    );
    match &s.arrangement {
        None => base,
        Some(a) => crate::export::scene::arrangement::resolve_arrangement(
            a,
            base,
            &layout_for(ma, ow, oh),
            &overlay_for(ma, ow, oh, true),
            sw,
            sh,
        ),
    }
}

struct Seg {
    start_ms: u32,
    end_ms: u32,
    scene: Scene,
    transition_ms: u32,
    easing: Easing,
    transition_out_ms: u32,
    easing_out: Easing,
}

pub struct LayoutTrack {
    segs: Vec<Seg>,
    base: Scene,
}

impl LayoutTrack {
    pub fn new(
        actions: &[ActionEvent],
        app: &AppearanceSettings,
        ow: u32,
        oh: u32,
        sw: u32,
        sh: u32,
        transition_ms: u32,
    ) -> Self {
        let scene_for = |id: LayoutId| {
            let ma = app.for_id(id);
            resolve(
                id,
                &layout_for(ma, ow, oh),
                &overlay_for(ma, ow, oh, true),
                sw,
                sh,
            )
        };
        let mut pts: Vec<(u32, LayoutId)> = vec![(0, LayoutId::Screen)];
        for a in actions {
            if let ActionKind::SetLayout(id) = a.kind {
                pts.push((a.t, id));
            }
        }
        let segs = pts
            .iter()
            .enumerate()
            .map(|(i, &(start, id))| {
                let end = pts.get(i + 1).map(|&(s, _)| s).unwrap_or(u32::MAX);
                let (tin, _) = fit_durations(
                    if i == 0 { 0 } else { transition_ms },
                    0,
                    end.saturating_sub(start),
                );
                Seg {
                    start_ms: start,
                    end_ms: end,
                    scene: scene_for(id),
                    transition_ms: tin,
                    easing: Easing::Smooth,
                    transition_out_ms: 0,
                    easing_out: Easing::Smooth,
                }
            })
            .collect();
        Self {
            segs,
            base: scene_for(LayoutId::Screen),
        }
    }

    pub fn from_segs(
        segs: &[crate::edit::model::LayoutSeg],
        app: &AppearanceSettings,
        ow: u32,
        oh: u32,
        sw: u32,
        sh: u32,
    ) -> Self {
        let scene_for = |id: LayoutId| {
            let ma = app.for_id(id);
            resolve(
                id,
                &layout_for(ma, ow, oh),
                &overlay_for(ma, ow, oh, true),
                sw,
                sh,
            )
        };
        let mut segs: Vec<Seg> = segs
            .iter()
            .map(|s| {
                let (tin, tout) = fit_durations(
                    s.transition_ms,
                    s.transition_out_ms,
                    s.end_ms.saturating_sub(s.start_ms),
                );
                Seg {
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    scene: resolve_seg_scene(s, app, ow, oh, sw, sh),
                    transition_ms: tin,
                    easing: crate::export::render::fromedit::easing_from(&s.easing, Easing::Smooth),
                    transition_out_ms: tout,
                    easing_out: crate::export::render::fromedit::easing_from(
                        &s.easing_out,
                        Easing::Smooth,
                    ),
                }
            })
            .collect();
        segs.sort_by_key(|s| s.start_ms);
        Self {
            segs,
            base: scene_for(LayoutId::Screen),
        }
    }

    fn active_idx(&self, t: u32) -> Option<usize> {
        self.segs
            .iter()
            .enumerate()
            .filter(|(_, s)| t >= s.start_ms && t < s.end_ms)
            .map(|(i, _)| i)
            .last()
    }

    fn raw_scene(&self, t: u32) -> Scene {
        self.active_idx(t)
            .map(|i| self.segs[i].scene)
            .unwrap_or(self.base)
    }

    fn successor(&self, end_ms: u32) -> (Scene, bool) {
        match self.active_idx(end_ms) {
            None => (self.base, false),
            Some(j) => {
                let s = &self.segs[j];
                (
                    s.scene,
                    s.transition_ms > 0 && end_ms.saturating_sub(s.start_ms) < s.transition_ms,
                )
            }
        }
    }

    pub fn scene_at(&self, t_ms: u32) -> Scene {
        match self.active_idx(t_ms) {
            None => self.base,
            Some(i) => {
                let s = &self.segs[i];
                let elapsed = t_ms.saturating_sub(s.start_ms);
                if s.transition_ms > 0 && elapsed < s.transition_ms {
                    let from = self.raw_scene(s.start_ms.saturating_sub(1));
                    let f = ease(s.easing, elapsed as f32 / s.transition_ms as f32);
                    return Scene::lerp(&from, &s.scene, f);
                }
                let exit_from = s.end_ms.saturating_sub(s.transition_out_ms);
                if s.transition_out_ms > 0 && t_ms >= exit_from {
                    let (to, next_entry_wins) = self.successor(s.end_ms);
                    if !next_entry_wins {
                        let f = ease(
                            s.easing_out,
                            (t_ms - exit_from) as f32 / s.transition_out_ms as f32,
                        );
                        return Scene::lerp(&s.scene, &to, f);
                    }
                }
                s.scene
            }
        }
    }
}

pub fn anchor_frame(raw: &[ZoomRegion], scene: &Scene, out: &mut Vec<ZoomRegion>) {
    out.clear();
    out.extend(raw.iter().map(|r| ZoomRegion {
        anchor: to_panel(r.anchor, scene.src, scene.screen.rect),
        ..*r
    }));
}

#[cfg(test)]
#[path = "layout_fit_tests.rs"]
mod fit_tests;
#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
