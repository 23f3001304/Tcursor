use crate::edit::text::{TextAnchor, TextAnim, TextItem};
use crate::export::camera::ease;
use crate::export::fx::text::text_style::{fill_rgb, style_of};
use crate::export::render::fromedit::easing_from;
use crate::export::types::Easing;

pub use crate::export::fx::caption::captionlayout::{ADV, LINE_H};

pub const SUB_RATIO: f32 = 0.58;
pub const PAD_X: f32 = 0.60;
pub const PAD_Y: f32 = 0.34;
pub const MARGIN: f32 = 0.060;
pub const RULE_W: f32 = 0.006;
pub const SLIDE_FRAC: f32 = 0.040;
pub const POP_FROM: f32 = 0.86;

#[derive(Clone, Debug, PartialEq)]
pub struct LaidText {
    pub main: String,
    pub sub: Option<String>,
    pub font_px: f32,
    pub sub_px: f32,
    pub plate: [f32; 4],
    pub main_baseline: [f32; 2],
    pub sub_baseline: [f32; 2],
    pub rule: [f32; 4],
    pub alpha: f32,
    pub reveal: usize,
    pub scale: f32,
    pub shift: [f32; 2],
    pub fill: [u8; 3],
    pub shadow: bool,
    pub plate_rgb: [u8; 3],
    pub plate_alpha: f32,
    pub rule_rgb: [u8; 3],
    pub centred: bool,
}

pub fn texts_at(items: &[TextItem], accent: [u8; 3], ow: u32, oh: u32, t_ms: u32) -> Vec<LaidText> {
    items
        .iter()
        .filter_map(|i| lay_one(i, accent, ow, oh, t_ms))
        .collect()
}

fn anchor_frac(a: TextAnchor) -> (f32, f32) {
    use TextAnchor::*;
    let ax = match a {
        TopLeft | MidLeft | BottomLeft => 0.0,
        TopCenter | MidCenter | BottomCenter => 0.5,
        _ => 1.0,
    };
    let ay = match a {
        TopLeft | TopCenter | TopRight => 0.0,
        MidLeft | MidCenter | MidRight => 0.5,
        _ => 1.0,
    };
    (ax, ay)
}

fn slide_dir(a: TextAnchor) -> (f32, f32) {
    use TextAnchor::*;
    match a {
        TopLeft | TopCenter | TopRight => (0.0, -1.0),
        BottomLeft | BottomCenter | BottomRight => (0.0, 1.0),
        MidLeft => (-1.0, 0.0),
        MidRight => (1.0, 0.0),
        MidCenter => (0.0, 0.0),
    }
}

fn place(a: f32, span: f32, block: f32, margin: f32) -> f32 {
    if a == 0.0 {
        margin
    } else if a == 1.0 {
        span - block - margin
    } else {
        (span - block) * 0.5
    }
}

pub fn lay_one(item: &TextItem, accent: [u8; 3], ow: u32, oh: u32, t_ms: u32) -> Option<LaidText> {
    if t_ms < item.start_ms || t_ms >= item.end_ms {
        return None;
    }
    let main = item.text.trim().to_string();
    let sub = item
        .sub
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if main.is_empty() && sub.is_none() {
        return None;
    }
    let (ow, oh) = (ow as f32, oh as f32);
    let e = easing_from(&item.easing, Easing::Smooth);
    let p = ((t_ms - item.start_ms) as f32 / item.in_ms.max(1) as f32).clamp(0.0, 1.0);
    let q = ((item.end_ms - t_ms) as f32 / item.out_ms.max(1) as f32).clamp(0.0, 1.0);
    let (e_in, e_out) = (ease(e, p), ease(e, q));
    let g = e_in.min(e_out);
    let anim = if e_in <= e_out {
        item.anim_in
    } else {
        item.anim_out
    };

    let st = style_of(&item.style);
    let scale = if anim == TextAnim::Pop {
        POP_FROM + (1.0 - POP_FROM) * g
    } else {
        1.0
    };
    let font_px = (oh * item.size.frac()).max(8.0) * scale;
    let sub_px = font_px * SUB_RATIO;
    let main_chars = main.chars().count();
    let sub_chars = sub.as_ref().map(|s| s.chars().count()).unwrap_or(0);
    let text_w = (main_chars as f32 * font_px * ADV).max(sub_chars as f32 * sub_px * ADV);
    let pad_x = if st.plate { 2.0 * font_px * PAD_X } else { 0.0 };
    let rule_w = if st.rule { oh * RULE_W } else { 0.0 };
    let block_w = text_w + pad_x + rule_w * 2.0;
    let block_h = font_px * LINE_H
        + sub.as_ref().map(|_| sub_px * LINE_H).unwrap_or(0.0)
        + if st.plate { 2.0 * font_px * PAD_Y } else { 0.0 };

    let (ax, ay) = anchor_frac(item.pos);
    let margin = oh * MARGIN;
    let (dx, dy) = slide_dir(item.pos);
    let slide = if anim == TextAnim::Slide {
        (1.0 - g) * oh * SLIDE_FRAC
    } else {
        0.0
    };
    let shift = [slide * dx, slide * dy];
    let bx = place(ax, ow, block_w, margin) + item.offset[0] * ow + shift[0];
    let by = place(ay, oh, block_h, margin) + item.offset[1] * oh + shift[1];

    let text_x = bx
        + pad_x * 0.5
        + if st.rule && ax <= 0.5 {
            rule_w * 2.0
        } else {
            0.0
        };
    let text_top = by + if st.plate { font_px * PAD_Y } else { 0.0 };
    Some(LaidText {
        main,
        sub,
        font_px,
        sub_px,
        plate: if st.plate {
            [bx, by, block_w, block_h]
        } else {
            [0.0; 4]
        },
        main_baseline: [text_x, text_top + font_px],
        sub_baseline: [text_x, text_top + font_px * LINE_H + sub_px],
        rule: if st.rule {
            [
                if ax > 0.5 { bx + block_w - rule_w } else { bx },
                by,
                rule_w,
                block_h,
            ]
        } else {
            [0.0; 4]
        },
        alpha: if anim == TextAnim::Typewriter {
            (if p > 0.0 { 1.0 } else { 0.0 }) * (q * 4.0).min(1.0)
        } else {
            g
        },
        reveal: if item.anim_in == TextAnim::Typewriter {
            (p * main_chars as f32).floor() as usize
        } else {
            usize::MAX
        },
        scale,
        shift,
        fill: fill_rgb(st.fill, accent),
        shadow: st.shadow,
        plate_rgb: st.plate_rgb,
        plate_alpha: st.plate_alpha,
        rule_rgb: accent,
        centred: ax == 0.5,
    })
}

#[cfg(test)]
#[path = "textlayout_tests.rs"]
mod tests;
