use crate::export::types::{Background, Rgb};

pub fn render(bg: &Background, w: u32, h: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    match bg {
        Background::Solid(c) => fill(&mut buf, w, h, |_, _| *c),
        Background::Image(_) => { // M2b stub: treat as solid dark; image library is M4
            fill(&mut buf, w, h, |_, _| Rgb { r: 24, g: 24, b: 30 });
        }
        Background::Gradient { from, to, angle_deg } => {
            let rad = angle_deg.to_radians();
            let (dx, dy) = (rad.cos(), rad.sin());
            let max = ((w as f32 - 1.0) * dx.abs()) + ((h as f32 - 1.0) * dy.abs());
            let max = max.max(1.0);
            fill(&mut buf, w, h, |x, y| {
                let t = ((x as f32 * dx) + (y as f32 * dy)).abs() / max;
                lerp(*from, *to, t.clamp(0.0, 1.0))
            });
        }
    }
    buf
}

fn fill(buf: &mut [u8], w: u32, h: u32, f: impl Fn(u32, u32) -> Rgb) {
    for y in 0..h {
        for x in 0..w {
            let c = f(x, y);
            let i = ((y * w + x) * 4) as usize;
            buf[i] = c.b; buf[i + 1] = c.g; buf[i + 2] = c.r; buf[i + 3] = 255;
        }
    }
}
fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Rgb { r: m(a.r, b.r), g: m(a.g, b.g), b: m(a.b, b.b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{Background, Rgb};
    #[test]
    fn solid_fills_bgra() {
        let buf = render(&Background::Solid(Rgb { r: 10, g: 20, b: 30 }), 2, 2);
        assert_eq!(buf.len(), 2 * 2 * 4);
        assert_eq!(&buf[0..4], &[30, 20, 10, 255]); // BGRA
    }
    #[test]
    fn gradient_differs_corner_to_corner() {
        let g = Background::Gradient { from: Rgb { r: 0, g: 0, b: 0 }, to: Rgb { r: 255, g: 255, b: 255 }, angle_deg: 0.0 };
        let buf = render(&g, 4, 1);
        assert!(buf[0] < buf[(3 * 4) as usize]); // left darker than right at 0deg
    }
}
