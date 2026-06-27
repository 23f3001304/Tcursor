use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::{ZoomConfig, ZoomRegion};

pub fn generate(events: &[MouseEvent], screen: &ScreenInfo, cfg: &ZoomConfig) -> Vec<ZoomRegion> {
    let downs: Vec<&MouseEvent> = events.iter().filter(|e| matches!(e.kind, EventKind::Down)).collect();
    let need = cfg.clicks_to_trigger.max(1) as usize;
    let mut regions = Vec::new();
    let mut i = 0;
    while i < downs.len() {
        // Require `need` clicks within merge_window_ms to TRIGGER a zoom, so a higher
        // "clicks to zoom" needs a quick multi-click rather than a single click.
        if i + need <= downs.len()
            && downs[i + need - 1].t.saturating_sub(downs[i].t) <= cfg.merge_window_ms
        {
            let first = downs[i];
            let mut last_t = downs[i + need - 1].t;
            let mut j = i + need;
            // Extend the hold: further clicks within idle_release_ms keep one steady zoom.
            while j < downs.len() && downs[j].t.saturating_sub(last_t) <= cfg.idle_release_ms {
                last_t = downs[j].t;
                j += 1;
            }
            regions.push(ZoomRegion {
                start_ms: first.t,
                end_ms: last_t + cfg.idle_release_ms + cfg.zoom_out_ms,
                zoom_in_ms: cfg.zoom_in_ms, zoom_out_ms: cfg.zoom_out_ms,
                target_scale: cfg.target_scale, anchor: to_frame(screen, first.x, first.y), easing: cfg.easing,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};
    use crate::export::types::ZoomConfig;
    fn down(t: u32, x: i32, y: i32) -> MouseEvent { MouseEvent { t, kind: EventKind::Down, x, y, button: Some(Button::Left) } }

    #[test]
    fn two_far_apart_clicks_make_two_regions() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(5000, 1500, 900)];
        let r = generate(&ev, &s, &cfg);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].anchor, crate::export::types::FramePoint { x: 100, y: 100 });
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].end_ms, 1000 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn nearby_rapid_clicks_merge_into_one_region() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 500, 500), down(1300, 520, 510), down(1600, 540, 505)];
        let r = generate(&ev, &s, &cfg);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].end_ms, 1600 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn click_while_zoomed_extends_regardless_of_distance() {
        // Already zoomed: a far-away click soon after must extend the SAME zoom
        // (continue + push the release later), not start a new one.
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(1500, 1800, 1000)]; // 1700px apart, 500ms later
        let r = generate(&ev, &s, &cfg);
        assert_eq!(r.len(), 1, "far-but-soon click should extend the active zoom");
        assert_eq!(r[0].anchor, crate::export::types::FramePoint { x: 100, y: 100 });
        assert_eq!(r[0].end_ms, 1500 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn idle_clicks_release_between_clusters() {
        // Clicks far apart in time produce separate, bounded zooms (each releases).
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(9000, 1500, 900)];
        let r = generate(&ev, &s, &cfg);
        assert_eq!(r.len(), 2);
        // first zoom ends well before the second begins -> there is a zoomed-out gap
        assert!(r[0].end_ms < r[1].start_ms);
    }

    #[test]
    fn clicks_to_trigger_two_needs_a_quick_double_click() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let mut cfg = ZoomConfig::default(); cfg.clicks_to_trigger = 2;
        // a lone click -> no zoom
        assert!(generate(&[down(1000, 100, 100)], &s, &cfg).is_empty());
        // two clicks within merge_window_ms (600) -> one region anchored at the first
        let ev = vec![down(1000, 100, 100), down(1300, 110, 105)];
        let r = generate(&ev, &s, &cfg);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].anchor, crate::export::types::FramePoint { x: 100, y: 100 });
    }

    #[test]
    fn clicks_to_trigger_two_ignores_slow_clicks() {
        let s = ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 };
        let mut cfg = ZoomConfig::default(); cfg.clicks_to_trigger = 2;
        // two clicks but > merge_window_ms apart -> not a continuous double-click -> no zoom
        let ev = vec![down(1000, 100, 100), down(3000, 100, 100)];
        assert!(generate(&ev, &s, &cfg).is_empty());
    }
}
