use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;
use crate::export::types::{ZoomConfig, ZoomRegion};

fn extend_hold(start: u32, acts: &[u32], idle_ms: u32) -> u32 {
    let mut last = start;
    for &t in acts {
        if t < last {
            continue;
        }
        if t.saturating_sub(last) <= idle_ms {
            last = t;
        } else {
            break;
        }
    }
    last
}

pub fn generate(
    events: &[MouseEvent],
    screen: &ScreenInfo,
    cfg: &ZoomConfig,
    typing: &[u32],
    smart: bool,
) -> Vec<ZoomRegion> {
    let downs: Vec<&MouseEvent> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Down))
        .collect();
    let need = cfg.clicks_to_trigger.max(1) as usize;
    let mut regions = Vec::new();
    let mut i = 0;
    while i < downs.len() {
        if i + need <= downs.len()
            && downs[i + need - 1].t.saturating_sub(downs[i].t) <= cfg.merge_window_ms
        {
            let first = downs[i];
            let cluster_last = downs[i + need - 1].t;
            let mut acts: Vec<u32> = downs[i + need..].iter().map(|d| d.t).collect();
            if smart {
                acts.extend(typing.iter().copied());
            }
            acts.retain(|&t| t >= cluster_last);
            acts.sort_unstable();
            let last_t = extend_hold(cluster_last, &acts, cfg.idle_release_ms);
            let mut j = i + need;
            while j < downs.len() && downs[j].t <= last_t {
                j += 1;
            }
            regions.push(ZoomRegion {
                start_ms: first.t,
                end_ms: last_t + cfg.idle_release_ms + cfg.zoom_out_ms,
                zoom_in_ms: cfg.zoom_in_ms,
                zoom_out_ms: cfg.zoom_out_ms,
                target_scale: cfg.target_scale,
                anchor: to_frame(screen, first.x, first.y),
                easing: cfg.easing,
                easing_out: cfg.easing,
                layer: 0,
                cam_action: None,
                follow_cursor: false,
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
    fn down(t: u32, x: i32, y: i32) -> MouseEvent {
        MouseEvent {
            t,
            kind: EventKind::Down,
            x,
            y,
            button: Some(Button::Left),
        }
    }

    #[test]
    fn two_far_apart_clicks_make_two_regions() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(5000, 1500, 900)];
        let r = generate(&ev, &s, &cfg, &[], false);
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0].anchor,
            crate::export::types::FramePoint { x: 100, y: 100 }
        );
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].end_ms, 1000 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn nearby_rapid_clicks_merge_into_one_region() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![
            down(1000, 500, 500),
            down(1300, 520, 510),
            down(1600, 540, 505),
        ];
        let r = generate(&ev, &s, &cfg, &[], false);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(r[0].end_ms, 1600 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn click_while_zoomed_extends_regardless_of_distance() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(1500, 1800, 1000)];
        let r = generate(&ev, &s, &cfg, &[], false);
        assert_eq!(
            r.len(),
            1,
            "far-but-soon click should extend the active zoom"
        );
        assert_eq!(
            r[0].anchor,
            crate::export::types::FramePoint { x: 100, y: 100 }
        );
        assert_eq!(r[0].end_ms, 1500 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn idle_clicks_release_between_clusters() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100), down(9000, 1500, 900)];
        let r = generate(&ev, &s, &cfg, &[], false);
        assert_eq!(r.len(), 2);
        assert!(r[0].end_ms < r[1].start_ms);
    }

    #[test]
    fn clicks_to_trigger_two_needs_a_quick_double_click() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let mut cfg = ZoomConfig::default();
        cfg.clicks_to_trigger = 2;
        assert!(generate(&[down(1000, 100, 100)], &s, &cfg, &[], false).is_empty());
        let ev = vec![down(1000, 100, 100), down(1300, 110, 105)];
        let r = generate(&ev, &s, &cfg, &[], false);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start_ms, 1000);
        assert_eq!(
            r[0].anchor,
            crate::export::types::FramePoint { x: 100, y: 100 }
        );
    }

    #[test]
    fn clicks_to_trigger_two_ignores_slow_clicks() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let mut cfg = ZoomConfig::default();
        cfg.clicks_to_trigger = 2;
        let ev = vec![down(1000, 100, 100), down(3000, 100, 100)];
        assert!(generate(&ev, &s, &cfg, &[], false).is_empty());
    }

    #[test]
    fn typing_extends_the_hold_when_smart() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100)];
        let typing = vec![1500u32, 3000, 4500];
        let r = generate(&ev, &s, &cfg, &typing, true);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].end_ms, 4500 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }

    #[test]
    fn typing_ignored_when_not_smart() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        };
        let cfg = ZoomConfig::default();
        let ev = vec![down(1000, 100, 100)];
        let r = generate(&ev, &s, &cfg, &[1500, 3000, 4500], false);
        assert_eq!(r[0].end_ms, 1000 + cfg.idle_release_ms + cfg.zoom_out_ms);
    }
}
