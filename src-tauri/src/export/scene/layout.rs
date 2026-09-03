use crate::actions::model::{ActionEvent, ActionKind};
use crate::actions::model::LayoutId;
use crate::export::camera::ease; // smoothstep + real ease-out-back spring (matches the zoom feel
                                 // and the preview's layoutAt mirror); export/easing::ease is
                                 // ease-out-cubic with spring==smooth, which would make the
                                 // LayoutInspector's Spring curve a silent no-op and diverge
                                 // from the preview.
use crate::export::coordmap::to_panel;
use crate::export::scene::{resolve, Scene};
use crate::export::types::{Easing, ZoomRegion};
use crate::edit::model::LayoutSeg;
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

/// One `LayoutSeg`'s resolved `Scene`: its POSES when it carries an `arrangement` (the `layout`
/// name still picks the appearance block the panels' radius/ring/shape come from), else its
/// preset's `Scene` directly. The single definition both `LayoutTrack::from_segs` (the export/
/// timeline path) and `preview_layouts` (the editor's per-segment preview rects) resolve a
/// segment through - factored out so a posed segment's live preview and what the export actually
/// draws can never diverge onto two pose-math paths.
pub fn resolve_seg_scene(s: &LayoutSeg, app: &AppearanceSettings, ow: u32, oh: u32, sw: u32, sh: u32) -> Scene {
    let id = crate::export::render::fromedit::layout_id_from(&s.layout);
    let ma = app.for_id(id);
    let base = resolve(id, &layout_for(ma, ow, oh), &overlay_for(ma, ow, oh, true), sw, sh);
    match &s.arrangement {
        None => base,
        Some(a) => crate::export::scene::arrangement::resolve_arrangement(
            a, base, &layout_for(ma, ow, oh), &overlay_for(ma, ow, oh, true), sw, sh),
    }
}

/// One layout segment active over `[start_ms, end_ms)`, with its own cross-fade feel in AND out.
struct Seg { start_ms: u32, end_ms: u32, scene: Scene, transition_ms: u32, easing: Easing,
    transition_out_ms: u32, easing_out: Easing }

/// Resolves the active `Scene` at any time. A segment is active only inside `[start, end)`;
/// OUTSIDE every segment (a gap, or before the first / after the last) falls back to the base
/// `screen` layout - the free-pill "empty means default" model. When multiple segments overlap a
/// time, the latest-starting one wins. On entering a segment, it cross-fades from whatever was
/// active just before it, over that segment's own `transition_ms` + `easing`.
pub struct LayoutTrack { segs: Vec<Seg>, base: Scene }

impl LayoutTrack {
    /// Fallback path: recorded `SetLayout` actions (persistent switch-points, so CONTIGUOUS
    /// segments that tile the whole timeline - no gaps) with a single global transition + Smooth.
    pub fn new(actions: &[ActionEvent], app: &AppearanceSettings, ow: u32, oh: u32,
               sw: u32, sh: u32, transition_ms: u32) -> Self {
        let scene_for = |id: LayoutId| {
            let ma = app.for_id(id);
            resolve(id, &layout_for(ma, ow, oh), &overlay_for(ma, ow, oh, true), sw, sh)
        };
        // Switch points, always starting at (0, Screen); each runs until the next switch.
        let mut pts: Vec<(u32, LayoutId)> = vec![(0, LayoutId::Screen)];
        for a in actions {
            if let ActionKind::SetLayout(id) = a.kind { pts.push((a.t, id)); }
        }
        let segs = pts.iter().enumerate().map(|(i, &(start, id))| {
            let end = pts.get(i + 1).map(|&(s, _)| s).unwrap_or(u32::MAX);
            Seg { start_ms: start, end_ms: end, scene: scene_for(id),
                transition_ms: if i == 0 { 0 } else { transition_ms }, easing: Easing::Smooth,
                transition_out_ms: 0, easing_out: Easing::Smooth }
        }).collect();
        Self { segs, base: scene_for(LayoutId::Screen) }
    }

    /// Edited path: one segment per `LayoutSeg`, each carrying its own `[start, end)` span +
    /// transition_ms + easing. Gaps between segments fall back to the base `screen`.
    ///
    /// Each segment resolves via `resolve_seg_scene` - once, here - `scene_at` only ever blends
    /// already-resolved scenes, so arrangement<->preset cross-fades come out of the same
    /// `Scene::lerp` as preset<->preset ones with no extra path.
    pub fn from_segs(segs: &[crate::edit::model::LayoutSeg], app: &AppearanceSettings,
                     ow: u32, oh: u32, sw: u32, sh: u32) -> Self {
        let scene_for = |id: LayoutId| {
            let ma = app.for_id(id);
            resolve(id, &layout_for(ma, ow, oh), &overlay_for(ma, ow, oh, true), sw, sh)
        };
        let mut segs: Vec<Seg> = segs.iter().map(|s| Seg {
            start_ms: s.start_ms, end_ms: s.end_ms,
            scene: resolve_seg_scene(s, app, ow, oh, sw, sh),
            transition_ms: s.transition_ms,
            easing: crate::export::render::fromedit::easing_from(&s.easing, Easing::Smooth),
            transition_out_ms: s.transition_out_ms,
            easing_out: crate::export::render::fromedit::easing_from(&s.easing_out, Easing::Smooth),
        }).collect();
        segs.sort_by_key(|s| s.start_ms);
        Self { segs, base: scene_for(LayoutId::Screen) }
    }

    /// Index of the active segment at `t` (last-starting one containing `t`), or `None` in a gap.
    fn active_idx(&self, t: u32) -> Option<usize> {
        self.segs.iter().enumerate().filter(|(_, s)| t >= s.start_ms && t < s.end_ms).map(|(i, _)| i).last()
    }

    /// The active layout scene at `t` WITHOUT any cross-fade (for computing a fade's "from").
    fn raw_scene(&self, t: u32) -> Scene {
        self.active_idx(t).map(|i| self.segs[i].scene).unwrap_or(self.base)
    }

    /// What this segment hands off to at its `end_ms` - the next segment if the two are gapless,
    /// else the base `screen` - plus whether that successor's OWN entry blend is still running at
    /// that instant. When it is, the successor's entry WINS: only one blend may be in flight, so
    /// the exit stands down rather than double-blending against it (which is what keeps gapless
    /// back-to-back segments bit-identical to their pre-exit-transition behavior).
    fn successor(&self, end_ms: u32) -> (Scene, bool) {
        match self.active_idx(end_ms) {
            None => (self.base, false),
            Some(j) => {
                let s = &self.segs[j];
                (s.scene, s.transition_ms > 0 && end_ms.saturating_sub(s.start_ms) < s.transition_ms)
            }
        }
    }

    /// Scene at `t_ms`: the active segment (else base `screen`), cross-faded from whatever was
    /// active just before its start over its own transition_ms/easing, and - over the last
    /// `transition_out_ms` before its end - toward whatever follows it, reaching that successor
    /// exactly AT `end_ms`. The entry is checked first, so a segment shorter than its own two
    /// transitions still resolves deterministically.
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
                        let f = ease(s.easing_out, (t_ms - exit_from) as f32 / s.transition_out_ms as f32);
                        return Scene::lerp(&s.scene, &to, f);
                    }
                }
                s.scene
            }
        }
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
#[path = "layout_tests.rs"]
mod tests;
