#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapturedCursor {
    pub w: u32,
    pub h: u32,
    pub hx: u32,
    pub hy: u32,
    pub rgba: Vec<u8>,
}

fn row(y: u32, h: u32, w: u32, top_down: bool) -> usize {
    let ry = if top_down {
        y
    } else {
        h.saturating_sub(1).saturating_sub(y)
    };
    ry as usize * w as usize * 4
}

fn mask_set(px: &[u8]) -> bool {
    px[0] | px[1] | px[2] != 0
}

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
