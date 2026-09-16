use crate::export::grade::{apply_px, GradeParams};

pub fn draw_grade(out: &mut [u8], ow: u32, oh: u32, p: &GradeParams) {
    if ow == 0 || oh == 0 {
        return;
    }
    let (fw, fh) = (ow as f32, oh as f32);
    for y in 0..oh {
        let v = (y as f32 + 0.5) / fh - 0.5;
        for x in 0..ow {
            let u = (x as f32 + 0.5) / fw - 0.5;
            let i = ((y * ow + x) * 4) as usize;
            let c = apply_px(
                [
                    out[i + 2] as f32 / 255.0,
                    out[i + 1] as f32 / 255.0,
                    out[i] as f32 / 255.0,
                ],
                p,
                u,
                v,
            );
            out[i] = (c[2] * 255.0).round() as u8;
            out[i + 1] = (c[1] * 255.0).round() as u8;
            out[i + 2] = (c[0] * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
#[path = "gradedraw_tests.rs"]
mod tests;
