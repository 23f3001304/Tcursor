// Pure cursor-bitmap conversion. `cursorcapture` hands Win32's 32bpp DIB rows to the functions
// here; they turn them into the straight-alpha, top-down RGBA a PNG (and the export blit) wants.
// No Win32 in this file on purpose - every rule below is a byte-slice transform with unit tests.

/// One captured OS cursor bitmap: straight-alpha RGBA, top-down, plus its hotspot in pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapturedCursor {
    pub w: u32,
    pub h: u32,
    pub hx: u32,
    pub hy: u32,
    pub rgba: Vec<u8>,
}

/// Byte offset of row `y` in a `w`-wide 32bpp DIB of `h` rows. A DIB is bottom-up by default
/// (row 0 is the BOTTOM scanline); `GetDIBits` only returns top-down rows when asked with a
/// negative height, so the order is a property of the read and has to travel with the bytes.
fn row(y: u32, h: u32, w: u32, top_down: bool) -> usize {
    let ry = if top_down { y } else { h.saturating_sub(1).saturating_sub(y) };
    ry as usize * w as usize * 4
}

/// Whether this AND-mask pixel is set (bit 1 = "leave the screen alone" = transparent). The 1bpp
/// mask is read back AS 32bpp, so bit 1 arrives white and bit 0 black - any lit channel means 1.
fn mask_set(px: &[u8]) -> bool {
    px[0] | px[1] | px[2] != 0
}

/// Colour cursor: `bgra` is `hbmColor` read as 32bpp, `and32` is `hbmMask` read the same way.
/// A 32bpp cursor whose alpha channel is entirely zero carries no real alpha (its shape lives in
/// the AND mask instead), so alpha is derived from the mask in that case - without this such a
/// cursor decodes fully transparent, i.e. invisible.
pub fn color_rgba(bgra: &[u8], and32: &[u8], w: u32, h: u32, top_down: bool) -> Option<Vec<u8>> {
    let need = w as usize * h as usize * 4;
    if need == 0 || bgra.len() < need {
        return None;
    }
    let real_alpha = bgra[..need].chunks_exact(4).any(|p| p[3] != 0);
    let masked = !real_alpha && and32.len() >= need;
    let mut out = vec![0u8; need];
    for y in 0..h {
        let off = row(y, h, w, top_down);
        for x in 0..w as usize {
            let s = &bgra[off + x * 4..];
            let a = if real_alpha {
                s[3]
            } else if masked && mask_set(&and32[off + x * 4..]) {
                0
            } else {
                255
            };
            let d = (y as usize * w as usize + x) * 4;
            out[d] = s[2];
            out[d + 1] = s[1];
            out[d + 2] = s[0];
            out[d + 3] = a;
        }
    }
    Some(out)
}

/// Monochrome cursor (`hbmColor` is NULL - the I-beam is the classic case): `mask32` is the
/// `2h`-tall `hbmMask` read as 32bpp, AND mask on top and XOR mask below. The Win32 rules are
/// AND=0,XOR=0 -> opaque black; AND=0,XOR=1 -> opaque white; AND=1,XOR=0 -> transparent;
/// AND=1,XOR=1 -> invert the screen. There is no inversion in a pre-composited sprite, so an
/// invert pixel is rendered opaque white (documented simplification - it is what the cursor looks
/// like over the dark surfaces those pixels are there for).
pub fn mono_rgba(mask32: &[u8], w: u32, h: u32, top_down: bool) -> Option<Vec<u8>> {
    let need = w as usize * h as usize * 4;
    if need == 0 || mask32.len() < need * 2 {
        return None;
    }
    let mut out = vec![0u8; need];
    for y in 0..h {
        let a_off = row(y, h * 2, w, top_down);
        let x_off = row(y + h, h * 2, w, top_down);
        for x in 0..w as usize {
            let and = mask_set(&mask32[a_off + x * 4..]);
            let xor = mask_set(&mask32[x_off + x * 4..]);
            let px: [u8; 4] = match (and, xor) {
                (true, false) => [0, 0, 0, 0],
                (false, false) => [0, 0, 0, 255],
                _ => [255, 255, 255, 255],
            };
            let d = (y as usize * w as usize + x) * 4;
            out[d..d + 4].copy_from_slice(&px);
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "cursorpixels_tests.rs"]
mod tests;
