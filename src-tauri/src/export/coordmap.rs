use crate::events::model::ScreenInfo;
use crate::export::types::Camera;
use crate::export::types::{FramePoint, Layout, RectF};

pub fn to_frame(screen: &ScreenInfo, x: i32, y: i32) -> FramePoint {
    FramePoint {
        x: x - screen.origin_x,
        y: y - screen.origin_y,
    }
}

pub fn inset_rect(sw: u32, sh: u32, layout: &Layout) -> (u32, u32, u32, u32) {
    let aw = layout.out_w.saturating_sub(2 * layout.pad_px).max(1) as f32;
    let ah = layout.out_h.saturating_sub(2 * layout.pad_px).max(1) as f32;
    let sa = sw.max(1) as f32 / sh.max(1) as f32;
    let (mut iw, mut ih) = if aw / ah > sa {
        (ah * sa, ah)
    } else {
        (aw, aw / sa)
    };
    let s = layout.screen_scale.clamp(0.1, 1.0);
    iw *= s;
    ih *= s;
    let ix = (layout.out_w as f32 - iw) / 2.0;
    let iy = (layout.out_h as f32 - ih) / 2.0;
    (
        ix.round() as u32,
        iy.round() as u32,
        iw.round() as u32,
        ih.round() as u32,
    )
}

pub fn corner_radius(layout: &Layout, iw: u32, ih: u32) -> f32 {
    layout.screen_radius_px.min(iw.min(ih) as f32 / 2.0)
}

pub fn to_base(p: FramePoint, sw: u32, sh: u32, layout: &Layout) -> FramePoint {
    let (ix, iy, iw, ih) = inset_rect(sw, sh, layout);
    let bx = ix as f32 + (p.x as f32 / sw.max(1) as f32) * iw as f32;
    let by = iy as f32 + (p.y as f32 / sh.max(1) as f32) * ih as f32;
    FramePoint {
        x: bx.round() as i32,
        y: by.round() as i32,
    }
}

pub fn full_src(sw: u32, sh: u32) -> RectF {
    RectF {
        x: 0.0,
        y: 0.0,
        w: sw.max(1) as f32,
        h: sh.max(1) as f32,
    }
}

pub fn to_panel(p: FramePoint, src: RectF, rect: RectF) -> FramePoint {
    let bx = rect.x + ((p.x as f32 - src.x) / src.w.max(1.0)) * rect.w;
    let by = rect.y + ((p.y as f32 - src.y) / src.h.max(1.0)) * rect.h;
    FramePoint {
        x: bx.round() as i32,
        y: by.round() as i32,
    }
}

pub fn crop(cam: Camera, ow: u32, oh: u32) -> (f32, f32, f32, f32) {
    let cw = (ow as f32 / cam.scale).round().max(1.0);
    let ch = (oh as f32 / cam.scale).round().max(1.0);
    let cx0 = (cam.cx - cw / 2.0).clamp(0.0, (ow as f32 - cw).max(0.0));
    let cy0 = (cam.cy - ch / 2.0).clamp(0.0, (oh as f32 - ch).max(0.0));
    (cx0, cy0, cw, ch)
}

pub fn project(bx: f32, by: f32, cam: Camera, ow: u32, oh: u32) -> (f32, f32) {
    let (cx0, cy0, cw, ch) = crop(cam, ow, oh);
    ((bx - cx0) * ow as f32 / cw, (by - cy0) * oh as f32 / ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::ScreenInfo;
    use crate::export::types::Layout;

    #[test]
    fn maps_screen_to_frame_minus_origin() {
        let s = ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 100,
            origin_y: 50,
        };
        let p = to_frame(&s, 300, 200);
        assert_eq!((p.x, p.y), (200, 150));
    }

    #[test]
    fn to_base_centers_screen_and_insets_corner() {
        let layout = Layout {
            out_w: 1920,
            out_h: 1080,
            pad_px: 60,
            screen_scale: 1.0,
            screen_radius_px: 1080.0 * 0.016,
        };
        let c = to_base(FramePoint { x: 960, y: 540 }, 1920, 1080, &layout);
        assert!((c.x - 960).abs() <= 1 && (c.y - 540).abs() <= 1);
        let tl = to_base(FramePoint { x: 0, y: 0 }, 1920, 1080, &layout);
        assert!(tl.x >= 60 && tl.y >= 60);
    }

    #[test]
    fn to_panel_maps_into_rect() {
        use crate::export::types::RectF;
        let r = RectF {
            x: 100.0,
            y: 50.0,
            w: 800.0,
            h: 600.0,
        };
        let c = to_panel(FramePoint { x: 960, y: 540 }, full_src(1920, 1080), r);
        assert!((c.x - 500).abs() <= 1 && (c.y - 350).abs() <= 1);
        let tl = to_panel(FramePoint { x: 0, y: 0 }, full_src(1920, 1080), r);
        assert_eq!((tl.x, tl.y), (100, 50));
    }

    #[test]
    fn a_full_canvas_src_reproduces_the_old_formula() {
        use crate::export::types::RectF;
        let r = RectF {
            x: 17.0,
            y: 42.0,
            w: 813.0,
            h: 611.0,
        };
        let (sw, sh) = (1920u32, 1080u32);
        for (x, y) in [(0, 0), (1, 1), (960, 540), (1919, 1079), (123, 977)] {
            let got = to_panel(FramePoint { x, y }, full_src(sw, sh), r);
            let old_x = r.x + (x as f32 / sw as f32) * r.w;
            let old_y = r.y + (y as f32 / sh as f32) * r.h;
            assert_eq!(
                (got.x, got.y),
                (old_x.round() as i32, old_y.round() as i32),
                "at {x},{y}"
            );
        }
    }

    #[test]
    fn a_span_src_maps_the_fitted_rect_onto_the_whole_panel() {
        use crate::export::types::RectF;
        let src = RectF {
            x: 96.0,
            y: 0.0,
            w: 1728.0,
            h: 1080.0,
        };
        let r = RectF {
            x: 200.0,
            y: 100.0,
            w: 1000.0,
            h: 625.0,
        };
        assert_eq!(
            (
                to_panel(FramePoint { x: 96, y: 0 }, src, r).x,
                to_panel(FramePoint { x: 96, y: 0 }, src, r).y
            ),
            (200, 100)
        );
        let c = to_panel(FramePoint { x: 960, y: 540 }, src, r);
        assert!(
            (c.x - 700).abs() <= 1 && (c.y - 412).abs() <= 1,
            "centre -> panel centre, got {c:?}"
        );
        assert_eq!(to_panel(FramePoint { x: 1824, y: 1080 }, src, r).x, 1200);
    }

    #[test]
    fn project_is_identity_at_scale_one() {
        use crate::export::types::Camera;
        let cam = Camera {
            cx: 960.0,
            cy: 540.0,
            scale: 1.0,
        };
        let p = project(100.0, 200.0, cam, 1920, 1080);
        assert!((p.0 - 100.0).abs() < 0.5 && (p.1 - 200.0).abs() < 0.5);
    }

    #[test]
    fn project_maps_crop_corner_to_origin_at_scale_two() {
        use crate::export::types::Camera;
        let cam = Camera {
            cx: 960.0,
            cy: 540.0,
            scale: 2.0,
        };
        let c = project(960.0, 540.0, cam, 1920, 1080);
        assert!((c.0 - 960.0).abs() < 1.0 && (c.1 - 540.0).abs() < 1.0);
        let tl = project(480.0, 270.0, cam, 1920, 1080);
        assert!(tl.0.abs() < 1.0 && tl.1.abs() < 1.0);
    }

    #[test]
    fn inset_scales_about_center_with_screen_scale() {
        let mut layout = Layout {
            out_w: 1920,
            out_h: 1080,
            pad_px: 0,
            screen_scale: 1.0,
            screen_radius_px: 0.0,
        };
        let (_, _, fw, fh) = inset_rect(1920, 1080, &layout);
        layout.screen_scale = 0.5;
        let (hx, hy, hw, hh) = inset_rect(1920, 1080, &layout);
        assert!((hw as f32 - fw as f32 * 0.5).abs() <= 1.0);
        assert!((hh as f32 - fh as f32 * 0.5).abs() <= 1.0);
        assert!((hx as f32 - (1920.0 - hw as f32) / 2.0).abs() <= 1.0);
        assert!((hy as f32 - (1080.0 - hh as f32) / 2.0).abs() <= 1.0);
    }
    #[test]
    fn corner_radius_uses_layout_value_and_clamps() {
        let layout = Layout {
            out_w: 1000,
            out_h: 1000,
            pad_px: 0,
            screen_scale: 1.0,
            screen_radius_px: 40.0,
        };
        assert_eq!(corner_radius(&layout, 400, 300), 40.0);
        assert_eq!(corner_radius(&layout, 50, 60), 25.0);
    }

    #[test]
    fn vertical_aspect_letterboxes_a_16x9_source_never_exceeding_the_frame() {
        use crate::export::types::Aspect;
        let mut layout = Layout::default();
        layout.apply_aspect(Aspect::Vertical9x16, 1920, 1080);
        assert_eq!((layout.out_w, layout.out_h), (1080, 1920));
        let (ix, iy, iw, ih) = inset_rect(1920, 1080, &layout);
        assert!(
            iw <= layout.out_w && ih <= layout.out_h,
            "inset must fit inside the frame"
        );
        assert!(
            ix + iw <= layout.out_w && iy + ih <= layout.out_h,
            "inset must not overflow the frame"
        );
        assert!(
            iw < layout.out_w,
            "expected pillarboxing, not an exact-width fill"
        );
    }
}
