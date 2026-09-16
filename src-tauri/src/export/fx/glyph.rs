use ab_glyph::{point, Font, FontRef, Glyph, PxScale, ScaleFont};

pub const FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

pub fn font() -> Option<FontRef<'static>> {
    FontRef::try_from_slice(FONT).ok()
}

pub fn run_width(font: &FontRef, px: f32, text: &str) -> f32 {
    let s = font.as_scaled(PxScale::from(px));
    text.chars().map(|c| s.h_advance(font.glyph_id(c))).sum()
}

#[allow(clippy::too_many_arguments)]
pub fn draw_run(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    font: &FontRef,
    px: f32,
    x: f32,
    y: f32,
    text: &str,
    rgb: [u8; 3],
    alpha: f32,
    shadow: bool,
) {
    if text.is_empty() || alpha <= 0.0 {
        return;
    }
    let s = font.as_scaled(PxScale::from(px));
    let mut pen = x;
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        let g: Glyph = gid.with_scale_and_position(px, point(pen, y));
        if let Some(og) = font.outline_glyph(g) {
            let bb = og.px_bounds();
            og.draw(|gx, gy, cov| {
                let bx = bb.min.x as i32 + gx as i32;
                let by = bb.min.y as i32 + gy as i32;
                if shadow {
                    put(out, ow, oh, bx + 1, by + 1, [0, 0, 0], cov * alpha * 0.6);
                }
                put(out, ow, oh, bx, by, rgb, cov * alpha);
            });
        }
        pen += s.h_advance(gid);
    }
}

pub fn put(out: &mut [u8], ow: u32, oh: u32, x: i32, y: i32, c: [u8; 3], a: f32) {
    if x < 0 || y < 0 || x as u32 >= ow || y as u32 >= oh {
        return;
    }
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let i = ((y as u32 * ow + x as u32) * 4) as usize;
    out[i] = (c[2] as f32 * a + out[i] as f32 * (1.0 - a)).round() as u8;
    out[i + 1] = (c[1] as f32 * a + out[i + 1] as f32 * (1.0 - a)).round() as u8;
    out[i + 2] = (c[0] as f32 * a + out[i + 2] as f32 * (1.0 - a)).round() as u8;
}

#[cfg(test)]
#[path = "glyph_tests.rs"]
mod tests;
