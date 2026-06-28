use crate::actions::model::{ActionEvent, ActionKind};
use crate::actions::model::LayoutId;
use crate::export::coordmap::to_panel;
use crate::export::easing::ease;
use crate::export::scene::{resolve, Scene};
use crate::export::types::{Easing, ZoomRegion};
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

/// Resolves the active `Scene` at any time from the recording's `SetLayout`
/// actions, cross-fading from the previous preset over `transition_ms` (eased).
pub struct LayoutTrack {
    switches: Vec<(u32, Scene)>, // (start_ms, scene) in time order; always starts at (0, ScreenFocus)
    transition_ms: u32,
}

impl LayoutTrack {
    pub fn new(actions: &[ActionEvent], app: &AppearanceSettings, ow: u32, oh: u32,
               sw: u32, sh: u32, transition_ms: u32) -> Self {
        let scene_for = |id: LayoutId| {
            let ma = app.for_id(id);
            resolve(id, &layout_for(ma, ow, oh), &overlay_for(ma, ow, oh, true), sw, sh)
        };
        let mut switches = vec![(0u32, scene_for(LayoutId::Screen))];
        for a in actions {
            if let ActionKind::SetLayout(id) = a.kind {
                switches.push((a.t, scene_for(id)));
            }
        }
        Self { switches, transition_ms }
    }

    /// Scene at `t_ms`: the last switch whose start <= t, cross-faded from the
    /// previous switch for the first `transition_ms` after the change.
    pub fn scene_at(&self, t_ms: u32) -> Scene {
        let i = self.switches.iter().rposition(|&(s, _)| s <= t_ms).unwrap_or(0);
        let (start, cur) = self.switches[i];
        if i == 0 || self.transition_ms == 0 { return cur; }
        let elapsed = t_ms.saturating_sub(start);
        if elapsed >= self.transition_ms { return cur; }
        let prev = self.switches[i - 1].1;
        let t = ease(Easing::Smooth, elapsed as f32 / self.transition_ms as f32);
        Scene::lerp(&prev, &cur, t)
    }
}

/// Re-anchor each zoom region into the screen panel active at the region's start
/// (anchors arrive in screen-local coords; the screen panel placement is layout-
/// and time-dependent). Identity-ish when the layout never changes.
pub fn anchor_regions(raw: Vec<ZoomRegion>, track: &LayoutTrack, sw: u32, sh: u32) -> Vec<ZoomRegion> {
    raw.into_iter()
        .map(|r| {
            let panel = track.scene_at(r.start_ms).screen.rect;
            ZoomRegion { anchor: to_panel(r.anchor, sw, sh, panel), ..r }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
    use crate::export::scene::{resolve, Scene};
    use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

    fn scene_of(app: &AppearanceSettings, id: LayoutId) -> Scene {
        let ma = app.for_id(id);
        resolve(id, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), 1920, 1080)
    }

    #[test]
    fn no_actions_is_screenfocus_everywhere() {
        let app = AppearanceSettings::default();
        let track = LayoutTrack::new(&[], &app, 3840, 2160, 1920, 1080, 350);
        let screen = scene_of(&app, LayoutId::Screen);
        assert_eq!(track.scene_at(0), screen);
        assert_eq!(track.scene_at(100_000), screen);
    }

    #[test]
    fn switch_transitions_then_settles() {
        let app = AppearanceSettings::default();
        let acts = vec![ActionEvent { t: 1000, kind: ActionKind::SetLayout(LayoutId::Camera) }];
        let track = LayoutTrack::new(&acts, &app, 3840, 2160, 1920, 1080, 400);
        let screen = scene_of(&app, LayoutId::Screen);
        let camera = scene_of(&app, LayoutId::Camera);
        assert_eq!(track.scene_at(999), screen);
        assert_eq!(track.scene_at(1000), screen);
        assert_eq!(track.scene_at(1400), camera);
        let mid = track.scene_at(1200);
        let (lo, hi) = (camera.screen.rect.w.min(screen.screen.rect.w), camera.screen.rect.w.max(screen.screen.rect.w));
        assert!(mid.screen.rect.w > lo && mid.screen.rect.w < hi);
    }

    #[test]
    fn latest_switch_wins() {
        let app = AppearanceSettings::default();
        let acts = vec![
            ActionEvent { t: 100, kind: ActionKind::SetLayout(LayoutId::Camera) },
            ActionEvent { t: 200, kind: ActionKind::SetLayout(LayoutId::Presenter) },
        ];
        let track = LayoutTrack::new(&acts, &app, 3840, 2160, 1920, 1080, 0);
        assert_eq!(track.scene_at(10_000), scene_of(&app, LayoutId::Presenter));
    }
}
