use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::settings::model::HotkeySettings;
use ab_glyph::{Font, FontRef, Glyph, point, PxScale, ScaleFont};

const FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

/// Caption lifetime in ms (moved from fxdraw).
const CAP_MS: u32 = 1300;

/// Draw the active caption (if `enabled`) onto the composited BGRA frame. Runs in
/// both FX render paths - captions are always a CPU ab_glyph blit.
pub fn overlay(out: &mut [u8], ow: u32, oh: u32, actions: &[crate::actions::model::ActionEvent],
    keys: &crate::settings::model::HotkeySettings, et: u32, enabled: bool) {
    if !enabled { return; }
    if let Some((text, a)) = caption_at(actions, keys, et, CAP_MS) {
        draw_caption(out, ow, oh, &text, a);
    }
}

/// The chord caption to show at event-time `et`, plus its fade alpha, or `None`.
/// The most-recent action within `life_ms` wins; a `ZoomHoldEnd` clears the caption.
pub fn caption_at(actions: &[ActionEvent], keys: &HotkeySettings, et: u32, life_ms: u32) -> Option<(String, f32)> {
    let a = actions.iter().filter(|a| a.t <= et && et - a.t < life_ms).next_back()?;
    let text = match a.kind {
        ActionKind::ZoomHoldEnd | ActionKind::SpotlightHoldEnd | ActionKind::VideoFxHoldEnd => return None,
        ActionKind::ZoomHoldStart => keys.zoom_hold.clone(),
        ActionKind::SpotlightHoldStart => keys.spotlight_hold.clone(),
        ActionKind::VideoFxHoldStart => keys.video_fx_hold.clone(),
        ActionKind::SetLayout(id) => match id {
            LayoutId::Screen => keys.layout_screen.clone(),
            LayoutId::Camera => keys.layout_camera.clone(),
            LayoutId::Presenter => keys.layout_presenter.clone(),
            LayoutId::ScreenOnly => keys.layout_screen_only.clone(),
            LayoutId::CameraOnly => keys.layout_camera_only.clone(),
        },
    };
    let p = (et - a.t) as f32 / life_ms.max(1) as f32;
    Some((text, (1.0 - p).clamp(0.0, 1.0)))
}

/// Render `text` centered in the bottom band of the BGRA frame, white with a 1px
/// dark shadow for legibility, blended at `alpha`. No-op on empty text / zero alpha.
pub fn draw_caption(out: &mut [u8], ow: u32, oh: u32, text: &str, alpha: f32) {
    if text.is_empty() || alpha <= 0.0 { return; }
    let font = match FontRef::try_from_slice(FONT) { Ok(f) => f, Err(_) => return };
    let px = (oh as f32 * 0.030).max(8.0);
    let scaled = font.as_scaled(PxScale::from(px));
    let width: f32 = text.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
    let mut x = (ow as f32 - width) / 2.0;
    let baseline = oh as f32 - oh as f32 * 0.06;
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        let g: Glyph = gid.with_scale_and_position(px, point(x, baseline));
        if let Some(og) = font.outline_glyph(g) {
            let bb = og.px_bounds();
            og.draw(|gx, gy, cov| {
                let bx = bb.min.x as i32 + gx as i32;
                let by = bb.min.y as i32 + gy as i32;
                put(out, ow, oh, bx + 1, by + 1, [0, 0, 0], cov * alpha * 0.6); // shadow
                put(out, ow, oh, bx, by, [255, 255, 255], cov * alpha);          // text
            });
        }
        x += scaled.h_advance(gid);
    }
}

/// Blend RGB `c` over the BGRA pixel at `(x,y)` if in bounds.
fn put(out: &mut [u8], ow: u32, oh: u32, x: i32, y: i32, c: [u8; 3], a: f32) {
    if x < 0 || y < 0 || x as u32 >= ow || y as u32 >= oh { return; }
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 { return; }
    let i = ((y as u32 * ow + x as u32) * 4) as usize;
    out[i] = (c[2] as f32 * a + out[i] as f32 * (1.0 - a)).round() as u8;
    out[i + 1] = (c[1] as f32 * a + out[i + 1] as f32 * (1.0 - a)).round() as u8;
    out[i + 2] = (c[0] as f32 * a + out[i + 2] as f32 * (1.0 - a)).round() as u8;
}

#[cfg(test)]
mod tests {
    use super::{caption_at, draw_caption};
    use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
    use crate::settings::model::HotkeySettings;

    fn keys() -> HotkeySettings { HotkeySettings::default() }

    #[test]
    fn shows_bound_chord_within_window() {
        let a = vec![ActionEvent { t: 100, kind: ActionKind::SetLayout(LayoutId::Screen) }];
        let (text, alpha) = caption_at(&a, &keys(), 200, 1300).unwrap();
        assert_eq!(text, keys().layout_screen);
        assert!(alpha > 0.0 && alpha <= 1.0);
    }

    #[test]
    fn none_after_window_and_for_hold_end() {
        let a = vec![ActionEvent { t: 0, kind: ActionKind::SetLayout(LayoutId::Camera) }];
        assert!(caption_at(&a, &keys(), 5000, 1300).is_none()); // expired
        let b = vec![ActionEvent { t: 0, kind: ActionKind::ZoomHoldEnd }];
        assert!(caption_at(&b, &keys(), 100, 1300).is_none()); // hold release clears
    }

    #[test]
    fn most_recent_action_wins() {
        let a = vec![
            ActionEvent { t: 0, kind: ActionKind::SetLayout(LayoutId::Screen) },
            ActionEvent { t: 100, kind: ActionKind::SetLayout(LayoutId::Presenter) },
        ];
        let (text, _) = caption_at(&a, &keys(), 150, 1300).unwrap();
        assert_eq!(text, keys().layout_presenter);
    }

    #[test]
    fn draw_caption_paints_pixels_and_noop_on_empty() {
        let (w, h) = (400u32, 200u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw_caption(&mut out, w, h, "Ctrl+Alt+1", 1.0);
        let lit = out.iter().filter(|&&b| b > 0).count();
        assert!(lit > 0, "caption should paint glyph pixels");
        let mut blank = vec![0u8; (w * h * 4) as usize];
        draw_caption(&mut blank, w, h, "", 1.0);
        assert!(blank.iter().all(|&b| b == 0), "empty text is a no-op");
    }
}
