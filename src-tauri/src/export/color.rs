#[inline]
pub fn yuv_to_rgb(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let l = 1.16438 * (y - 16.0);
    let cb = u - 128.0;
    let cr = v - 128.0;
    (
        clamp8(l + 1.59603 * cr),
        clamp8(l - 0.39176 * cb - 0.81297 * cr),
        clamp8(l + 2.01723 * cb),
    )
}

#[inline]
fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[inline]
fn uv_index(w: usize, h: usize, x: usize, y: usize) -> usize {
    w * h + (y / 2) * w + (x / 2) * 2
}

pub fn nv12_to_bgra(nv12: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let yv = nv12[y * w + x] as f32;
            let i = uv_index(w, h, x, y);
            let (r, g, b) = yuv_to_rgb(yv, nv12[i] as f32, nv12[i + 1] as f32);
            let o = (y * w + x) * 4;
            out[o] = b;
            out[o + 1] = g;
            out[o + 2] = r;
            out[o + 3] = 255;
        }
    }
    out
}

pub fn bgra_to_nv12(bgra: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h + (w * h) / 2];
    for y in 0..h {
        for x in 0..w {
            let o = (y * w + x) * 4;
            let (b, g, r) = (bgra[o] as f32, bgra[o + 1] as f32, bgra[o + 2] as f32);
            out[y * w + x] = clamp8(0.257 * r + 0.504 * g + 0.098 * b + 16.0);
            if y % 2 == 0 && x % 2 == 0 {
                let i = uv_index(w, h, x, y);
                out[i] = clamp8(-0.148 * r - 0.291 * g + 0.439 * b + 128.0);
                out[i + 1] = clamp8(0.439 * r - 0.368 * g - 0.071 * b + 128.0);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
