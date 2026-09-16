use super::captionlayout::{caption_at, layout, word_span};
use crate::edit::captions::Caption;
use crate::export::fx::glyph::{font as ui_font, put};
use crate::settings::captions::CaptionStyle;
use ab_glyph::{point, Font, FontRef, Glyph, PxScale, ScaleFont};

const RADIUS: f32 = 0.30;

struct Pen {
    px: f32,
    alpha: f32,
    text: [u8; 3],
    lit: [u8; 3],
    hi: Option<(usize, usize)>,
    cut: Option<(usize, usize)>,
    word_alpha: f32,
}

pub fn overlay(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    caps: &[Caption],
    style: &CaptionStyle,
    accent: [u8; 3],
    t_ms: u32,
) {
    if !style.enabled {
        return;
    }
    let Some(cap) = caption_at(caps, t_ms) else {
        return;
    };
    let l = layout(cap, style, ow, oh, t_ms);
    if l.alpha <= 0.0 || l.lines.is_empty() {
        return;
    }
    let Some(font) = ui_font() else {
        return;
    };
    if style.pill {
        let a = l.alpha * style.pill_alpha as f32 / 100.0;
        scrim(out, ow, oh, l.pill, a, style.pill_color);
    }
    let pen = Pen {
        px: l.font_px,
        alpha: l.alpha,
        text: style.text_color,
        lit: style.highlight_color.unwrap_or(accent),
        hi: l.hi.and_then(|i| word_span(cap, i)),
        cut: l.words_shown.map(|n| cut_at(cap, n)),
        word_alpha: l.word_alpha,
    };
    let cx = l.pill[0] + l.pill[2] / 2.0;
    let mut base = 0usize;
    for (i, line) in l.lines.iter().enumerate() {
        draw_line(out, ow, oh, &font, line, cx, l.baselines[i], &pen, base);
        base += line.chars().count() + 1;
    }
}

fn cut_at(cap: &Caption, shown: usize) -> (usize, usize) {
    match shown.checked_sub(1).and_then(|i| word_span(cap, i)) {
        Some(span) => span,
        None => (0, 0),
    }
}

fn scrim(out: &mut [u8], ow: u32, oh: u32, r: [f32; 4], a: f32, col: [u8; 3]) {
    let (x, y, w, h) = (r[0], r[1], r[2], r[3]);
    let rad = (h * RADIUS).min(w / 2.0);
    let (x0, y0) = ((x.floor() as i32).max(0), (y.floor() as i32).max(0));
    let (x1, y1) = (
        ((x + w).ceil() as i32).min(ow as i32),
        ((y + h).ceil() as i32).min(oh as i32),
    );
    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let dx = (x + rad - fx).max(fx - (x + w - rad)).max(0.0);
            let dy = (y + rad - fy).max(fy - (y + h - rad)).max(0.0);
            let cov = (rad + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            put(out, ow, oh, px, py, col, cov * a);
            let i = ((py as u32 * ow + px as u32) * 4 + 3) as usize;
            out[i] = out[i].max((cov * a * 255.0).round() as u8);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    font: &FontRef<'_>,
    line: &str,
    cx: f32,
    baseline: f32,
    pen: &Pen,
    base: usize,
) {
    let scaled = font.as_scaled(PxScale::from(pen.px));
    let width: f32 = line
        .chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum();
    let mut x = cx - width / 2.0;
    for (k, ch) in line.chars().enumerate() {
        let gid = font.glyph_id(ch);
        let g = base + k;
        // INVARIANT: an unrevealed glyph still advances, so the run never reflows.
        let alpha = match pen.cut {
            Some((_, e)) if g >= e => 0.0,
            Some((s, _)) if g >= s => pen.alpha * pen.word_alpha,
            _ => pen.alpha,
        };
        if alpha > 0.0 {
            let lit = pen.hi.is_some_and(|(s, e)| g >= s && g < e);
            let col = if lit { pen.lit } else { pen.text };
            let gl: Glyph = gid.with_scale_and_position(pen.px, point(x, baseline));
            if let Some(og) = font.outline_glyph(gl) {
                let bb = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    let bx = bb.min.x as i32 + gx as i32;
                    let by = bb.min.y as i32 + gy as i32;
                    put(out, ow, oh, bx + 1, by + 1, [0, 0, 0], cov * alpha * 0.6);
                    put(out, ow, oh, bx, by, col, cov * alpha);
                });
            }
        }
        x += scaled.h_advance(gid);
    }
}

#[cfg(test)]
#[path = "captiondraw_tests.rs"]
mod tests;
