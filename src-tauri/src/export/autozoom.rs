use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::{ZoomConfig, ZoomRegion};

pub fn generate(events: &[MouseEvent], screen: &ScreenInfo, cfg: &ZoomConfig) -> Vec<ZoomRegion> {
    let downs: Vec<&MouseEvent> = events.iter().filter(|e| matches!(e.kind, EventKind::Down)).collect();
    let mut regions = Vec::new();
    let mut i = 0;
    while i < downs.len() {
        let first = downs[i];
        let mut last_t = first.t;
        let mut j = i + 1;
        while j < downs.len() {
            let d = downs[j];
            let near_time = d.t.saturating_sub(last_t) <= cfg.merge_window_ms;
            let dx = (d.x - first.x).unsigned_abs();
            let dy = (d.y - first.y).unsigned_abs();
            let near_space = dx <= cfg.merge_radius_px && dy <= cfg.merge_radius_px;
            if near_time && near_space { last_t = d.t; j += 1; } else { break; }
        }
        regions.push(ZoomRegion {
            start_ms: first.t,
            end_ms: last_t + cfg.idle_release_ms + cfg.zoom_out_ms,
            zoom_in_ms: cfg.zoom_in_ms, zoom_out_ms: cfg.zoom_out_ms,
            target_scale: cfg.target_scale, anchor: to_frame(screen, first.x, first.y), easing: cfg.easing,
        });
        i = j;
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
}
