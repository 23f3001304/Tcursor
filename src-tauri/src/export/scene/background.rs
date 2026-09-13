use std::path::{Path, PathBuf};
use crate::export::types::{Background, Rgb};
use crate::settings::background::{BackgroundKind, BackgroundSettings};
use crate::settings::bg_asset;

/// Build the static export background buffer from user settings. `Mesh` decodes the bundled
/// wallpaper named by `settings.mesh` (`settings::wallpapers`, cover-fitted so 16:9 art doesn't
/// stretch), or - when that id is empty or unknown to this build - the legacy `mesh_jpg` exactly
/// as it always did (falling back to the gradient `Background::default()` if ffmpeg can't decode
/// it). `Solid`/`Gradient` render the matching `Background` variant directly, no subprocess.
/// `Image`/`Video` decode the user's imported file under `project_dir` (a `Video` contributes its
/// FIRST frame here; the moving picture is `pipeline::bg_pipe`'s job in the export and the TS
/// preview's in the editor).
/// Called once per export/preview build (`FrameRenderer::new`/`reload_edit`), never per frame,
/// so the decode, the optional blur, and the dim pass below are all cheap in practice.
pub fn build(settings: &BackgroundSettings, mesh_jpg: &[u8], w: u32, h: u32, project_dir: &Path) -> Vec<u8> {
    use crate::export::pipeline::ffio;
    let legacy = || ffio::decode_image(mesh_jpg, w, h).unwrap_or_else(|_| render(&Background::default(), w, h));
    // The background the user would see if the asset weren't there: their chosen wallpaper (or the
    // legacy mesh). Reached by a missing file, an unreadable one, and by both kinds when the
    // decode fails - a broken import must never cost the whole background, let alone the export.
    let base = || match crate::settings::wallpapers::wallpaper_by_id(&settings.mesh) {
        Some(wp) => ffio::decode_image_cover(wp.bytes, w, h).unwrap_or_else(|_| legacy()),
        None => legacy(),
    };
    let mut buf = match settings.kind {
        BackgroundKind::Mesh => base(),
        BackgroundKind::Image | BackgroundKind::Video => match asset_file(settings, project_dir) {
            Some(f) => ffio::decode_file_cover(&f, w, h).unwrap_or_else(|_| { warn_asset(&f.display()); base() }),
            None => { warn_asset(&settings.asset.as_deref().unwrap_or("(none)")); base() }
        },
        BackgroundKind::Solid => render(&Background::Solid(rgb(settings.solid)), w, h),
        BackgroundKind::Gradient => render(&Background::Gradient {
            from: rgb(settings.gradient_from), mid: settings.gradient_mid.map(rgb),
            to: rgb(settings.gradient_to), angle_deg: settings.gradient_angle_deg,
        }, w, h),
    };
    if settings.blur > 0.0 { blur(&mut buf, w, h, settings.blur); }
    apply_dim(&mut buf, settings.dim_clamped());
    buf
}

/// The absolute path of the imported asset, for the two kinds that have one.
fn asset_file(settings: &BackgroundSettings, project_dir: &Path) -> Option<PathBuf> {
    match settings.kind {
        BackgroundKind::Image | BackgroundKind::Video =>
            bg_asset::asset_path(project_dir, settings.asset.as_deref().unwrap_or("")),
        _ => None,
    }
}

/// The file the export should open a decode STREAM on: a `Video` background whose asset really
/// exists. `None` for every other kind (an image needs no stream) and for a missing file.
pub fn video_source(settings: &BackgroundSettings, project_dir: &Path) -> Option<PathBuf> {
    (settings.kind == BackgroundKind::Video).then(|| asset_file(settings, project_dir)).flatten()
}

/// Darken a BGRA buffer by `dim` (0..1), which is exactly a black overlay at that alpha:
/// `out = round(src * (1 - dim))`, alpha untouched. `src/editor/stage/stageBg.ts` paints the same
/// formula over the preview's own video draw, so the two agree; `dim == 0` touches nothing at all,
/// which is what keeps the default path free (this runs per FRAME for a video background).
pub fn apply_dim(buf: &mut [u8], dim: f32) {
    let k = 1.0 - dim.clamp(0.0, 1.0);
    if k >= 1.0 { return; }
    for px in buf.chunks_exact_mut(4) {
        for c in px.iter_mut().take(3) { *c = (*c as f32 * k).round() as u8; }
    }
}

/// Say ONCE per process that a background asset could not be used. Once, because this is called
/// from a per-frame-adjacent path and a broken import would otherwise flood the log.
fn warn_asset(what: &dyn std::fmt::Display) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| eprintln!("[BG] background asset unusable ({what}) - falling back to the wallpaper"));
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
        Background::Gradient { from, mid, to, angle_deg } => {
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
            // With a middle stop the ramp is two half-length lerps meeting at t = 0.5; without
            // one it is the single lerp it has always been, bit for bit.
            fill(&mut buf, w, h, |x, y| {
                let t = (((x as f32 * dx) + (y as f32 * dy) - pmin) / range).clamp(0.0, 1.0);
                match mid {
                    None => lerp(*from, *to, t),
                    Some(m) if t < 0.5 => lerp(*from, *m, t * 2.0),
                    Some(m) => lerp(*m, *to, (t - 0.5) * 2.0),
                }
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
#[path = "background_tests.rs"]
mod tests;
