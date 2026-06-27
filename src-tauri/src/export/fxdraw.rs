use crate::actions::model::ActionEvent;
use crate::events::model::MouseEvent;
use crate::export::clickfx::{fade_alpha, hits_at, ripple_radius};
use crate::export::coordmap::{project, to_panel};
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, HotkeySettings};

const LIFE_MS: u32 = 600;
const CAP_MS: u32 = 1300;

/// Draw the click-FX + spotlight + caption overlay onto the composited BGRA frame `out`.
/// `cur` is the cursor's base/scene point (pre-zoom) the exporter already computed.
/// Drawn for both compositors (it runs on the returned buffer), so CPU == GPU.
#[allow(clippy::too_many_arguments)]
pub fn overlay(
    out: &mut [u8], ow: u32, oh: u32, scene: &Scene, cam: Camera, cur: FramePoint,
    events: &[MouseEvent], sw: u32, sh: u32, et: u32, fx: &ClickFxSettings,
    actions: &[ActionEvent], keys: &HotkeySettings,
) {
    if !fx.enabled { return; }
    if fx.spotlight {
        let (cx, cy) = project(cur.x as f32, cur.y as f32, cam, ow, oh);
        spotlight(out, ow, oh, cx, cy, fx.intensity);
    }
    if !matches!(fx.style, ClickFxStyle::None) {
        let r_max = oh as f32 * 0.06;
        for h in hits_at(events, et, LIFE_MS) {
            let b = to_panel(FramePoint { x: h.sx, y: h.sy }, sw, sh, scene.screen.rect);
            let (px, py) = project(b.x as f32, b.y as f32, cam, ow, oh);
            let a = fade_alpha(h.progress, fx.intensity);
            match fx.style {
                ClickFxStyle::Ripple => ring(out, ow, oh, px, py, ripple_radius(h.progress, r_max), oh as f32 * 0.006, fx.color, a),
                ClickFxStyle::Pulse => disc(out, ow, oh, px, py, oh as f32 * 0.02, fx.color, a),
                ClickFxStyle::None => {}
            }
        }
    }
    if fx.captions {
        if let Some((text, a)) = crate::export::caption::caption_at(actions, keys, et, CAP_MS) {
            crate::export::caption::draw_caption(out, ow, oh, &text, a);
        }
    }
}

/// Alpha-blend RGB `c` over the BGRA pixel at byte index `i`.
fn blend(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 { return; }
    out[i] = (c[2] as f32 * a + out[i] as f32 * (1.0 - a)).round() as u8;         // B
    out[i + 1] = (c[1] as f32 * a + out[i + 1] as f32 * (1.0 - a)).round() as u8; // G
    out[i + 2] = (c[0] as f32 * a + out[i + 2] as f32 * (1.0 - a)).round() as u8; // R
}

/// Visit every pixel within `rad` of `(cx,cy)` with its distance, clipped to frame.
fn for_disc(ow: u32, oh: u32, cx: f32, cy: f32, rad: f32, mut f: impl FnMut(usize, f32)) {
    let r = rad.ceil() as i32 + 1;
    let (cxi, cyi) = (cx.round() as i32, cy.round() as i32);
    for y in (cyi - r).max(0)..(cyi + r).min(oh as i32) {
        for x in (cxi - r).max(0)..(cxi + r).min(ow as i32) {
            let d = (((x as f32 - cx).powi(2)) + ((y as f32 - cy).powi(2))).sqrt();
            f(((y as u32 * ow + x as u32) * 4) as usize, d);
        }
    }
}

fn ring(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, thick: f32, c: [u8; 3], a: f32) {
    let t = thick.max(1.0);
    for_disc(ow, oh, cx, cy, radius + t, |i, d| {
        let cov = (t - (d - radius).abs()).clamp(0.0, t) / t; // AA band falloff
        if cov > 0.0 { blend(out, i, c, a * cov); }
    });
}

fn disc(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32) {
    for_disc(ow, oh, cx, cy, radius, |i, d| {
        let cov = (radius - d).clamp(0.0, 1.0); // 1px AA edge
        if cov > 0.0 { blend(out, i, c, a * cov); }
    });
}

/// Darken the frame outside a soft radius around `(cx,cy)`; `intensity` deepens it.
fn spotlight(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, intensity: f32) {
    let (r_in, r_out) = (oh as f32 * 0.14, oh as f32 * 0.32);
    let dim = 0.55 * intensity.clamp(0.0, 1.0);
    for y in 0..oh {
        for x in 0..ow {
            let d = (((x as f32 - cx).powi(2)) + ((y as f32 - cy).powi(2))).sqrt();
            let t = ((d - r_in) / (r_out - r_in)).clamp(0.0, 1.0);
            let k = 1.0 - dim * t; // brightness multiplier
            if k < 1.0 {
                let i = ((y * ow + x) * 4) as usize;
                for c in 0..3 { out[i + c] = (out[i + c] as f32 * k).round() as u8; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind, MouseEvent};
    use crate::export::scene::{Panel, Scene};
    use crate::export::types::{Camera, FramePoint, RectF};
    use crate::settings::model::{ClickFxSettings, ClickFxStyle};

    fn frame(w: u32, h: u32) -> Vec<u8> { vec![0u8; (w * h * 4) as usize] }
    fn full_scene(w: u32, h: u32) -> Scene {
        Scene {
            screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: w as f32, h: h as f32 }, radius: 0.0, alpha: 1.0 },
            camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0 },
        }
    }
    fn fx(style: ClickFxStyle, spotlight: bool) -> ClickFxSettings {
        ClickFxSettings { enabled: true, style, color: [255, 0, 0], intensity: 1.0, captions: false, spotlight }
    }
    fn cam() -> Camera { Camera { cx: 50.0, cy: 50.0, scale: 1.0 } }

    #[test]
    fn ripple_paints_a_colored_ring() {
        let (w, h) = (100u32, 100u32);
        let mut out = frame(w, h);
        let ev = vec![MouseEvent { t: 0, kind: EventKind::Down, x: 50, y: 50, button: Some(Button::Left) }];
        // progress ~0.5 -> radius ~ h*0.06*0.5 = 3px. Sample a pixel on that ring.
        overlay(&mut out, w, h, &full_scene(w, h), cam(), FramePoint { x: 50, y: 50 }, &ev, w, h, 300, &fx(ClickFxStyle::Ripple, false), &[], &crate::settings::model::HotkeySettings::default());
        let painted = out.chunks(4).any(|p| p[2] > 40); // some red (R in BGRA index 2)
        assert!(painted, "ripple should paint red pixels");
    }

    #[test]
    fn disabled_leaves_frame_untouched() {
        let (w, h) = (40u32, 40u32);
        let mut out = frame(w, h);
        let ev = vec![MouseEvent { t: 0, kind: EventKind::Down, x: 20, y: 20, button: Some(Button::Left) }];
        let mut off = fx(ClickFxStyle::Ripple, true);
        off.enabled = false;
        overlay(&mut out, w, h, &full_scene(w, h), cam(), FramePoint { x: 20, y: 20 }, &ev, w, h, 100, &off, &[], &crate::settings::model::HotkeySettings::default());
        assert!(out.iter().all(|&b| b == 0), "disabled FX must not draw");
    }

    #[test]
    fn spotlight_dims_corner_more_than_center() {
        let (w, h) = (100u32, 100u32);
        let mut out = vec![200u8; (w * h * 4) as usize]; // bright grey
        overlay(&mut out, w, h, &full_scene(w, h), cam(), FramePoint { x: 50, y: 50 }, &[], w, h, 0, &fx(ClickFxStyle::None, true), &[], &crate::settings::model::HotkeySettings::default());
        let center = out[((50 * w + 50) * 4) as usize];
        let corner = out[0];
        assert!(corner < center, "corner must be dimmer than the lit center");
    }
}
