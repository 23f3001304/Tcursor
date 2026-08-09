use crate::export::types::{Background, Rgb};
use crate::settings::background::{BackgroundKind, BackgroundSettings};

/// Build the static export background buffer from user settings. `Mesh` decodes the bundled
/// `mesh_jpg` (falling back to the gradient `Background::default()` if ffmpeg can't decode it);
/// `Solid`/`Gradient` render the matching `Background` variant directly, no ffmpeg subprocess.
/// Called once per export/preview build (`FrameRenderer::new`/`reload_edit`), never per frame,
/// so the optional blur pass below is cheap even though it isn't itself per-pixel-parallel.
pub fn build(settings: &BackgroundSettings, mesh_jpg: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut buf = match settings.kind {
        BackgroundKind::Mesh => crate::export::pipeline::ffio::decode_image(mesh_jpg, w, h)
            .unwrap_or_else(|_| render(&Background::default(), w, h)),
        BackgroundKind::Solid => render(&Background::Solid(rgb(settings.solid)), w, h),
        BackgroundKind::Gradient => render(&Background::Gradient {
            from: rgb(settings.gradient_from), to: rgb(settings.gradient_to), angle_deg: settings.gradient_angle_deg,
        }, w, h),
    };
    if settings.blur > 0.0 { blur(&mut buf, w, h, settings.blur); }
    buf
}

fn rgb(c: [u8; 3]) -> Rgb { Rgb { r: c[0], g: c[1], b: c[2] } }

/// Separable box blur (horizontal pass, then vertical), O(w*h) via a sliding-window sum -
/// cheap enough to run once per background rebuild. `amount` (0..1) maps to a radius up to 3%
/// of the shorter side. Edge pixels clamp (no vignette darkening at the border).
fn blur(buf: &mut Vec<u8>, w: u32, h: u32, amount: f32) {
    let r = (amount.clamp(0.0, 1.0) * 0.03 * w.min(h) as f32).round() as i32;
    if r <= 0 { return; }
    let mid = blur_h(buf, w, h, r);
    *buf = blur_v(&mid, w, h, r);
}

/// One row at a time, sliding-window box sum across columns (edge-clamped reads).
fn blur_h(src: &[u8], w: u32, h: u32, r: i32) -> Vec<u8> {
    let (wi, hi) = (w as i32, h as i32);
    let mut out = vec![0u8; src.len()];
    for y in 0..hi {
        for c in 0..4usize {
            let at = |x: i32| src[((y * wi + x.clamp(0, wi - 1)) * 4) as usize + c] as i32;
            let mut sum: i32 = (-r..=r).map(at).sum();
            for x in 0..wi {
                out[((y * wi + x) * 4) as usize + c] = (sum / (2 * r + 1)) as u8;
                sum += at(x + r + 1) - at(x - r);
            }
        }
    }
    out
}

/// Same sliding-window box sum, down each column (edge-clamped reads).
fn blur_v(src: &[u8], w: u32, h: u32, r: i32) -> Vec<u8> {
    let (wi, hi) = (w as i32, h as i32);
    let mut out = vec![0u8; src.len()];
    for x in 0..wi {
        for c in 0..4usize {
            let at = |y: i32| src[((y.clamp(0, hi - 1) * wi + x) * 4) as usize + c] as i32;
            let mut sum: i32 = (-r..=r).map(at).sum();
            for y in 0..hi {
                out[((y * wi + x) * 4) as usize + c] = (sum / (2 * r + 1)) as u8;
                sum += at(y + r + 1) - at(y - r);
            }
        }
    }
    out
}

pub fn render(bg: &Background, w: u32, h: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (w * h * 4) as usize];
    match bg {
        Background::Solid(c) => fill(&mut buf, w, h, |_, _| *c),
        Background::Image(_) => { // M2b stub: treat as solid dark; image library is M4
            fill(&mut buf, w, h, |_, _| Rgb { r: 24, g: 24, b: 30 });
        }
        Background::Gradient { from, to, angle_deg } => {
            // Normalize against the projection's TRUE range over the four frame corners, not
            // `.abs()` of a single corner - `.abs()` mirror-folds any angle whose projection goes
            // negative (including the DEFAULT 135deg), putting a crease of `from` on the fold line
            // instead of a monotonic corner-to-corner ramp.
            let rad = angle_deg.to_radians();
            let (dx, dy) = (rad.cos(), rad.sin());
            let (wf, hf) = (w as f32 - 1.0, h as f32 - 1.0);
            let corners = [0.0, wf * dx, hf * dy, wf * dx + hf * dy];
            let pmin = corners.iter().cloned().fold(f32::INFINITY, f32::min);
            let pmax = corners.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let range = (pmax - pmin).max(1e-6);
            fill(&mut buf, w, h, |x, y| {
                let t = ((x as f32 * dx) + (y as f32 * dy) - pmin) / range;
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
    #[test]
    fn gradient_at_135deg_is_a_true_monotonic_ramp_not_mirror_folded() {
        // The DEFAULT angle (135deg, top-left -> bottom-right diagonal). The old
        // `.abs()`-normalized formula folded the ramp along y=x, putting a crease of `from`
        // there instead of a monotonic corner-to-corner ramp - this is exactly the angle that
        // regresses if normalization goes back to projecting-onto-`[0, max]` with an abs().
        let g = Background::Gradient { from: Rgb { r: 0, g: 0, b: 0 }, to: Rgb { r: 255, g: 255, b: 255 }, angle_deg: 135.0 };
        let (w, h) = (100u32, 100u32);
        let buf = render(&g, w, h);
        let px = |x: u32, y: u32| buf[((y * w + x) * 4) as usize]; // blue channel; from/to are gray
        let (top_right, center, bottom_left) = (px(99, 0), px(50, 50), px(0, 99));
        assert!(top_right < center, "top-right ({top_right}) must be darker than center ({center})");
        assert!(center < bottom_left, "center ({center}) must be darker than bottom-left ({bottom_left})");
        // The two corners perpendicular to the gradient axis sit at the diagonal's midpoint.
        assert_eq!(px(0, 0), px(99, 99), "the off-axis corners must be equal (both at t=0.5)");
    }

    // `build` tests only exercise Solid/Gradient (pure Rust, deterministic) - `Mesh` shells out
    // to ffmpeg and is intentionally left untested here, same as `render`'s Image stub above.
    #[test]
    fn build_solid_matches_direct_render() {
        use crate::settings::background::{BackgroundKind, BackgroundSettings};
        let s = BackgroundSettings { kind: BackgroundKind::Solid, solid: [10, 20, 30], ..Default::default() };
        let buf = build(&s, &[], 2, 2); // mesh bytes irrelevant for Solid
        assert_eq!(buf, render(&Background::Solid(Rgb { r: 10, g: 20, b: 30 }), 2, 2));
    }
    #[test]
    fn build_zero_blur_is_a_no_op() {
        use crate::settings::background::{BackgroundKind, BackgroundSettings};
        let s = BackgroundSettings { kind: BackgroundKind::Gradient, blur: 0.0, ..Default::default() };
        let buf = build(&s, &[], 16, 16);
        let direct = render(&Background::Gradient { from: rgb(s.gradient_from), to: rgb(s.gradient_to), angle_deg: s.gradient_angle_deg }, 16, 16);
        assert_eq!(buf, direct);
    }
    #[test]
    fn blur_softens_a_sharp_edge() {
        // 40x40 (not tiny): the blur radius is a fraction of `w.min(h)`, so a 1px-tall test
        // image would always round down to radius 0 - this size guarantees a non-zero radius.
        let (w, h) = (40u32, 40u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        for y in 0..h { for x in 20..w {
            let i = ((y * w + x) * 4) as usize;
            buf[i] = 255; buf[i + 1] = 255; buf[i + 2] = 255; buf[i + 3] = 255;
        } }
        blur(&mut buf, w, h, 1.0);
        // The pixel just left of the old hard edge (x=19) should have picked up some brightness -
        // a sharp 0/255 edge no longer jumps straight from black to white.
        let i = ((20 * w + 19) * 4) as usize;
        assert!(buf[i] > 0, "edge should have softened into the dark side");
    }
    #[test]
    fn blur_amount_zero_is_untouched() {
        let mut buf = vec![7u8, 8, 9, 255, 1, 2, 3, 255];
        let before = buf.clone();
        blur(&mut buf, 2, 1, 0.0);
        assert_eq!(buf, before);
    }
}
