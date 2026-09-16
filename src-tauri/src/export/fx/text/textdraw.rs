use crate::edit::text::TextItem;
use crate::export::fx::glyph;
use crate::export::fx::text::textlayout::{texts_at, LaidText, PAD_X};

const PLATE_RADIUS: f32 = 0.35;

fn rrect_sd(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32 {
    let (cx, cy) = ((mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5);
    let (hx, hy) = ((mx[0] - mn[0]) * 0.5 - r, (mx[1] - mn[1]) * 0.5 - r);
    let (qx, qy) = ((x - cx).abs() - hx, (y - cy).abs() - hy);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

pub fn overlay(out: &mut [u8], ow: u32, oh: u32, items: &[TextItem], accent: [u8; 3], t_ms: u32) {
    if items.is_empty() {
        return;
    }
    let laid = texts_at(items, accent, ow, oh, t_ms);
    if laid.is_empty() {
        return;
    }
    let Some(font) = glyph::font() else {
        return;
    };
    for l in &laid {
        if l.alpha <= 0.0 {
            continue;
        }
        fill_rect(
            out,
            ow,
            oh,
            l.plate,
            l.plate_rgb,
            l.plate_alpha * l.alpha,
            l.font_px * PLATE_RADIUS,
        );
        fill_rect(out, ow, oh, l.rule, l.rule_rgb, l.alpha, 0.0);
        draw_lines(out, ow, oh, &font, l);
    }
}

fn fill_rect(out: &mut [u8], ow: u32, oh: u32, r: [f32; 4], rgb: [u8; 3], a: f32, radius: f32) {
    if r[2] <= 0.0 || r[3] <= 0.0 || a <= 0.0 {
        return;
    }
    let mn = [r[0], r[1]];
    let mx = [r[0] + r[2], r[1] + r[3]];
    let rad = radius.min(r[2] * 0.5).min(r[3] * 0.5);
    let x0 = (r[0].floor().max(0.0) as u32).min(ow);
    let y0 = (r[1].floor().max(0.0) as u32).min(oh);
    let x1 = (mx[0].ceil().max(0.0) as u32).min(ow);
    let y1 = (mx[1].ceil().max(0.0) as u32).min(oh);
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = (0.5 - rrect_sd(x as f32 + 0.5, y as f32 + 0.5, mn, mx, rad)).clamp(0.0, 1.0);
            if cov > 0.0 {
                glyph::put(out, ow, oh, x as i32, y as i32, rgb, cov * a);
            }
        }
    }
}

fn run_x(font: &ab_glyph::FontRef, px: f32, text: &str, l: &LaidText) -> f32 {
    if !l.centred {
        return l.main_baseline[0];
    }
    let inner = if l.plate[2] > 0.0 {
        l.plate[2] - 2.0 * l.font_px * PAD_X
    } else {
        (l.main.chars().count() as f32).max(
            l.sub
                .as_ref()
                .map(|s| s.chars().count() as f32)
                .unwrap_or(0.0),
        ) * l.font_px
            * crate::export::fx::text::textlayout::ADV
    };
    l.main_baseline[0] + (inner - glyph::run_width(font, px, text)) * 0.5
}

fn draw_lines(out: &mut [u8], ow: u32, oh: u32, font: &ab_glyph::FontRef, l: &LaidText) {
    let shown: String = if l.reveal == usize::MAX {
        l.main.clone()
    } else {
        l.main.chars().take(l.reveal).collect()
    };
    if !shown.is_empty() {
        let x = run_x(font, l.font_px, &shown, l);
        glyph::draw_run(
            out,
            ow,
            oh,
            font,
            l.font_px,
            x,
            l.main_baseline[1],
            &shown,
            l.fill,
            l.alpha,
            l.shadow,
        );
    }
    if let Some(sub) = &l.sub {
        let x = run_x(font, l.sub_px, sub, l);
        glyph::draw_run(
            out,
            ow,
            oh,
            font,
            l.sub_px,
            x,
            l.sub_baseline[1],
            sub,
            l.fill,
            l.alpha * 0.82,
            l.shadow,
        );
    }
}

#[cfg(test)]
#[path = "textdraw_tests.rs"]
mod tests;
