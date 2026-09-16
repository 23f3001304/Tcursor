use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::export::fx::glyph;
use crate::settings::model::HotkeySettings;

const CAP_MS: u32 = 1300;

pub fn overlay(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    actions: &[crate::actions::model::ActionEvent],
    keys: &crate::settings::model::HotkeySettings,
    et: u32,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    if let Some((text, a)) = caption_at(actions, keys, et, CAP_MS) {
        draw_caption(out, ow, oh, &text, a);
    }
}

pub fn caption_at(
    actions: &[ActionEvent],
    keys: &HotkeySettings,
    et: u32,
    life_ms: u32,
) -> Option<(String, f32)> {
    let a = actions
        .iter()
        .filter(|a| a.t <= et && et - a.t < life_ms)
        .next_back()?;
    let text = match a.kind {
        ActionKind::ZoomHoldEnd | ActionKind::SpotlightHoldEnd | ActionKind::VideoFxHoldEnd => {
            return None
        }
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

pub fn draw_caption(out: &mut [u8], ow: u32, oh: u32, text: &str, alpha: f32) {
    if text.is_empty() || alpha <= 0.0 {
        return;
    }
    let Some(font) = glyph::font() else {
        return;
    };
    let px = (oh as f32 * 0.030).max(8.0);
    let x = (ow as f32 - glyph::run_width(&font, px, text)) / 2.0;
    let baseline = oh as f32 - oh as f32 * 0.06;
    glyph::draw_run(
        out,
        ow,
        oh,
        &font,
        px,
        x,
        baseline,
        text,
        [255, 255, 255],
        alpha,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::{caption_at, draw_caption};
    use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
    use crate::settings::model::HotkeySettings;

    fn keys() -> HotkeySettings {
        HotkeySettings::default()
    }

    #[test]
    fn shows_bound_chord_within_window() {
        let a = vec![ActionEvent {
            t: 100,
            kind: ActionKind::SetLayout(LayoutId::Screen),
        }];
        let (text, alpha) = caption_at(&a, &keys(), 200, 1300).unwrap();
        assert_eq!(text, keys().layout_screen);
        assert!(alpha > 0.0 && alpha <= 1.0);
    }

    #[test]
    fn none_after_window_and_for_hold_end() {
        let a = vec![ActionEvent {
            t: 0,
            kind: ActionKind::SetLayout(LayoutId::Camera),
        }];
        assert!(caption_at(&a, &keys(), 5000, 1300).is_none());
        let b = vec![ActionEvent {
            t: 0,
            kind: ActionKind::ZoomHoldEnd,
        }];
        assert!(caption_at(&b, &keys(), 100, 1300).is_none());
    }

    #[test]
    fn most_recent_action_wins() {
        let a = vec![
            ActionEvent {
                t: 0,
                kind: ActionKind::SetLayout(LayoutId::Screen),
            },
            ActionEvent {
                t: 100,
                kind: ActionKind::SetLayout(LayoutId::Presenter),
            },
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
