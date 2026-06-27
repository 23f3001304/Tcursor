use crate::actions::model::{ActionEvent, ActionKind};
use crate::actions::model::LayoutId;
use crate::export::coordmap::to_panel;
use crate::export::easing::ease;
use crate::export::scene::{resolve, Scene};
use crate::export::types::{Easing, Layout, OverlayLayout, ZoomRegion};

/// Resolves the active `Scene` at any time from the recording's `SetLayout`
/// actions, cross-fading from the previous preset over `transition_ms` (eased).
pub struct LayoutTrack {
    switches: Vec<(u32, Scene)>, // (start_ms, scene) in time order; always starts at (0, ScreenFocus)
    transition_ms: u32,
}

impl LayoutTrack {
    pub fn new(actions: &[ActionEvent], layout: &Layout, overlay: &OverlayLayout,
               sw: u32, sh: u32, transition_ms: u32) -> Self {
        let mut switches = vec![(0u32, resolve(LayoutId::Screen, layout, overlay, sw, sh))];
        for a in actions {
            if let ActionKind::SetLayout(id) = a.kind {
                switches.push((a.t, resolve(id, layout, overlay, sw, sh)));
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
    use crate::export::scene::resolve;
    use crate::export::types::{Layout, OverlayLayout};

    fn fixtures() -> (Layout, OverlayLayout) { (Layout::default(), OverlayLayout::default()) }

    #[test]
    fn no_actions_is_screenfocus_everywhere() {
        let (l, ov) = fixtures();
        let track = LayoutTrack::new(&[], &l, &ov, 1920, 1080, 350);
        let screen = resolve(LayoutId::Screen, &l, &ov, 1920, 1080);
        assert_eq!(track.scene_at(0), screen);
        assert_eq!(track.scene_at(100_000), screen);
    }

    #[test]
    fn switch_transitions_then_settles() {
        let (l, ov) = fixtures();
        let acts = vec![ActionEvent { t: 1000, kind: ActionKind::SetLayout(LayoutId::Camera) }];
        let track = LayoutTrack::new(&acts, &l, &ov, 1920, 1080, 400);
        let screen = resolve(LayoutId::Screen, &l, &ov, 1920, 1080);
        let camera = resolve(LayoutId::Camera, &l, &ov, 1920, 1080);
        assert_eq!(track.scene_at(999), screen);   // before the switch
        assert_eq!(track.scene_at(1000), screen);  // t=0 of transition == previous
        assert_eq!(track.scene_at(1400), camera);  // transition complete
        let mid = track.scene_at(1200);            // strictly between the two screen-panel widths
        let (lo, hi) = (camera.screen.rect.w.min(screen.screen.rect.w), camera.screen.rect.w.max(screen.screen.rect.w));
        assert!(mid.screen.rect.w > lo && mid.screen.rect.w < hi);
    }

    #[test]
    fn latest_switch_wins() {
        let (l, ov) = fixtures();
        let acts = vec![
            ActionEvent { t: 100, kind: ActionKind::SetLayout(LayoutId::Camera) },
            ActionEvent { t: 200, kind: ActionKind::SetLayout(LayoutId::Presenter) },
        ];
        let track = LayoutTrack::new(&acts, &l, &ov, 1920, 1080, 0); // 0 = no transition
        assert_eq!(track.scene_at(10_000), resolve(LayoutId::Presenter, &l, &ov, 1920, 1080));
    }
}
