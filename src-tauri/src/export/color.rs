// nv12 <-> BGRA color conversion for the export screen path. The screen decoder now emits nv12
// (Y plane + interleaved half-res UV, ~2.6x smaller than BGRA) instead of BGRA, so far fewer bytes
// cross the ffmpeg->exporter pipe (the measured export bottleneck). The GPU compositor converts
// nv12->RGB in the shader; this module is the CPU-side twin (fallback compositor + tests).
//
// Coefficients are BT.601 limited-range, which reproduces ffmpeg's default `-pix_fmt bgra`
// conversion for our "unknown" color-metadata captures - verified byte-exact (meanAbsDiff < 1,
// maxAbsDiff = 3, pure rounding) against ffmpeg on a real 4K recording. `gpu/shader.wgsl`'s
// `screen_rgb` MUST use these exact constants so the GPU and CPU paths agree.

/// One BT.601 limited-range YUV sample (0..255 each) to RGB (u8). Folded coefficients:
/// 1.16438 = 255/219 (luma), chroma terms pre-scaled by 255/224.
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
fn clamp8(v: f32) -> u8 { v.round().clamp(0.0, 255.0) as u8 }

/// nv12 layout offset of the UV pair covering pixel (x, y): `[U, V]` at `uv_index(..)` and `+1`.
#[inline]
fn uv_index(w: usize, h: usize, x: usize, y: usize) -> usize {
    w * h + (y / 2) * w + (x / 2) * 2
}

/// Convert a full `w x h` nv12 buffer (`w*h*3/2` bytes) to BGRA (`w*h*4` bytes). Used by the CPU
/// fallback compositor; the GPU path does the same math in the shader.
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

/// Convert BGRA (`w*h*4`) to nv12 (`w*h*3/2`), BT.601 limited-range. Only used to build nv12 test
/// inputs from the existing BGRA test fixtures; the real pipeline gets nv12 straight from ffmpeg.
pub fn bgra_to_nv12(bgra: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h + (w * h) / 2];
    for y in 0..h {
        for x in 0..w {
            let o = (y * w + x) * 4;
            let (b, g, r) = (bgra[o] as f32, bgra[o + 1] as f32, bgra[o + 2] as f32);
            out[y * w + x] = clamp8(0.257 * r + 0.504 * g + 0.098 * b + 16.0); // Y
            if y % 2 == 0 && x % 2 == 0 {
                let i = uv_index(w, h, x, y);
                out[i] = clamp8(-0.148 * r - 0.291 * g + 0.439 * b + 128.0); // U (Cb)
                out[i + 1] = clamp8(0.439 * r - 0.368 * g - 0.071 * b + 128.0); // V (Cr)
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
