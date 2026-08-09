use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::model::{MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::{FramePoint, ZoomConfig, ZoomRegion};

/// Screen-local cursor position at `t_ms` (last mouse sample at or before t), or
/// screen center if none. Used as the manual-zoom anchor at the key press.
/// Assumes `events` are in capture (time) order.
fn cursor_at(events: &[MouseEvent], screen: &ScreenInfo, t_ms: u32) -> FramePoint {
    let mut last: Option<&MouseEvent> = None;
    for e in events {
        if e.t <= t_ms { last = Some(e); } else { break; }
    }
    match last {
        Some(e) => to_frame(screen, e.x, e.y),
        None => FramePoint { x: screen.w as i32 / 2, y: screen.h as i32 / 2 },
    }
}

/// One manual zoom region per `ZoomHoldStart..ZoomHoldEnd` pair: zoom in at the
/// press location, hold (CameraSim box-follows the cursor), ease out after release.
/// Scale/timing come from settings (same `ZoomConfig` as click-zoom). Anchors are
/// screen-local; the exporter re-anchors them into the active screen panel.
pub fn from_actions(actions: &[ActionEvent], events: &[MouseEvent], screen: &ScreenInfo, cfg: &ZoomConfig) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    let mut start: Option<u32> = None;
    for a in actions {
        match a.kind {
            ActionKind::ZoomHoldStart => start = Some(a.t),
            ActionKind::ZoomHoldEnd => {
                if let Some(s) = start.take() {
                    regions.push(ZoomRegion {
                        start_ms: s,
                        end_ms: a.t.max(s + 1) + cfg.zoom_out_ms,
                        zoom_in_ms: cfg.zoom_in_ms,
                        zoom_out_ms: cfg.zoom_out_ms,
                        target_scale: cfg.target_scale,
                        anchor: cursor_at(events, screen, s),
                        easing: cfg.easing,
                        cam_action: None,
                        layer: 0,
                        follow_cursor: false,
                    });
                }
            }
            _ => {}
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::from_actions;
    use crate::actions::model::{ActionEvent, ActionKind};
    use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
    use crate::export::types::{FramePoint, ZoomConfig};

    fn mv(t: u32, x: i32, y: i32) -> MouseEvent { MouseEvent { t, kind: EventKind::Move, x, y, button: None } }
    fn act(t: u32, k: ActionKind) -> ActionEvent { ActionEvent { t, kind: k } }
    fn screen() -> ScreenInfo { ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 } }

    #[test]
    fn hold_pair_makes_one_region_anchored_at_press() {
        let cfg = ZoomConfig::default();
        let events = vec![mv(900, 500, 300), mv(1100, 700, 400)];
        let actions = vec![act(1000, ActionKind::ZoomHoldStart), act(2000, ActionKind::ZoomHoldEnd)];
        let r = from_actions(&actions, &events, &screen(), &cfg);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].end_ms, 2000 + cfg.zoom_out_ms); // release + ease-out
        assert_eq!(r[0].target_scale, cfg.target_scale);
        assert_eq!(r[0].anchor, FramePoint { x: 500, y: 300 }); // cursor sample at/<=press time
    }

    #[test]
    fn unpaired_start_makes_no_region() {
        let cfg = ZoomConfig::default();
        let actions = vec![act(1000, ActionKind::ZoomHoldStart)];
        assert!(from_actions(&actions, &[], &screen(), &cfg).is_empty());
    }

    #[test]
    fn two_holds_make_two_regions() {
        let cfg = ZoomConfig::default();
        let actions = vec![
            act(1000, ActionKind::ZoomHoldStart), act(1500, ActionKind::ZoomHoldEnd),
            act(3000, ActionKind::ZoomHoldStart), act(3500, ActionKind::ZoomHoldEnd),
        ];
        let r = from_actions(&actions, &[], &screen(), &cfg);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].start_ms, 3000);
        // no mouse samples -> anchor defaults to screen center
        assert_eq!(r[0].anchor, FramePoint { x: 960, y: 540 });
    }
}
