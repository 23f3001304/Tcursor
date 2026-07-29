// Render one composited frame at an arbitrary time T from edit.json.
// A WARM FrameRenderer (GPU pipeline, probed dims, decoded background, loaded
// events/edit/actions) is cached per recording in PreviewSession and reused across
// scrubs - only the per-frame seek-decode + composite + PNG-encode rerun. The cache is
// keyed by (folder, edit.json mtime, aspect), so an edit (which rewrites edit.json)
// transparently rebuilds it and the next preview reflects the change.
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::render::{FrameRenderer, RenderMeta, OUT_FPS};
use crate::export::settings::Resolution;
use crate::export::types::{Aspect, Layout};
use crate::session::paths::ProjectPaths;

/// Long edge (px) of the preview compositing canvas - independent of the proxy-video transcode
/// height (`ensure_proxy`'s `quality`, a separate concern). 1280 matches the old hardcoded 16:9
/// preview (1280x720) exactly, so a default `Source` aspect on a 16:9 recording is unchanged.
const PREVIEW_LONG_EDGE: u32 = 1280;

/// Build a fresh preview renderer downscaled to `PREVIEW_LONG_EDGE`, following the doc's chosen
/// aspect exactly (`Layout::resolve`) so the preview frame is always proportional to what export
/// would produce for the same recording + aspect.
fn build_renderer(paths: &ProjectPaths) -> Result<(FrameRenderer, RenderMeta)> {
    let fps = crate::win::sys::display::primary_refresh_hz().min(60);
    FrameRenderer::new(paths, Layout::default(), fps, Resolution::Source, Some(PREVIEW_LONG_EDGE))
}

/// Per-frame work given a warm renderer: rewind the camera, fast-forward to T (math only),
/// seek-decode the screen + webcam frame at T, composite, PNG-encode at the renderer's resolved size.
fn render_frame(renderer: &mut FrameRenderer, meta: &RenderMeta, paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>> {
    // step_camera requires ascending t; rewind so the cached renderer can re-scan to T.
    renderer.reset_camera();
    let k_target = time_ms as u64 * OUT_FPS / 1000;
    let mut pose = renderer.step_camera(meta.video_start);
    for j in 1..=k_target {
        pose = renderer.step_camera(meta.video_start + j * 1000 / OUT_FPS);
    }

    // Seek-decode one screen frame at time_ms (the screen file's frame 0 is video_start).
    let mut screen_buf = vec![0u8; meta.screen_bytes];
    let mut screen_dec = RawDecoder::spawn(
        &paths.video(), 0.0, false, Some(time_ms as u64), None, None, "nv12", meta.screen_bytes)?;
    if !screen_dec.read_frame(&mut screen_buf)? {
        anyhow::bail!("no screen frame at {time_ms}ms (past end of video)");
    }
    drop(screen_dec);

    // Seek-decode one webcam frame if present (export pre-seeks webcam by video_start).
    let wc_size = meta.webcam_size;
    let wc_bytes = (wc_size * wc_size * 4) as usize;
    let webcam: Option<(Vec<u8>, u32, u32)> = if paths.webcam().exists() {
        let mut buf = vec![0u8; wc_bytes];
        let mut wc_dec = RawDecoder::spawn(
            &paths.webcam(), OUT_FPS as f64, false,
            Some(meta.video_start + time_ms as u64), Some(wc_size), None, "bgra", wc_bytes)?;
        wc_dec.read_frame(&mut buf)?;
        drop(wc_dec);
        Some((buf, wc_size, wc_size))
    } else {
        None
    };

    let wc_ref = webcam.as_ref().map(|(b, w, h)| (b.as_slice(), *w, *h));
    let mut bgra = Vec::new();
    renderer.composite_at(&pose, &screen_buf, wc_ref, &mut bgra);
    png_encode(&bgra, meta.out_w, meta.out_h)
}

/// Render one composited frame at `time_ms` into the recording at `paths` (uncached:
/// builds a fresh renderer). Returns PNG bytes at the resolved preview size.
pub fn render_preview(paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>> {
    let (mut renderer, meta) = build_renderer(paths)?;
    render_frame(&mut renderer, &meta, paths, time_ms)
}

/// A warm preview renderer cached for one recording + edit revision.
pub(crate) struct Cached { pub folder: String, pub mtime: Option<SystemTime>, pub aspect: Aspect, pub renderer: FrameRenderer, pub meta: RenderMeta }

/// Managed Tauri state: the most-recently-used warm preview renderer (one at a time).
#[derive(Default)]
pub struct PreviewSession(Mutex<Option<Cached>>);

/// Run `f` with the warm renderer for `folder`, (re)building it when the folder or the doc's
/// aspect changes - both resize the frame, so the cached GPU compositor/background/FX (sized for
/// the OLD dims) cannot just refresh. A same-aspect `edit.json` change instead calls the cheap
/// `FrameRenderer::reload_edit`. The single place the preview cache is keyed - shared by every
/// preview command (frame, camera track, layout, clicks, background) so the warm-up logic lives once.
pub(crate) fn with_warm<T>(session: &PreviewSession, folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>) -> Result<T, String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    let mtime = std::fs::metadata(paths.edit()).and_then(|m| m.modified()).ok();
    let mut guard = session.0.lock().unwrap();
    let fresh = matches!(guard.as_ref(), Some(c) if c.folder == folder && c.mtime == mtime);
    if !fresh {
        let same_folder = matches!(guard.as_ref(), Some(c) if c.folder == folder);
        let aspect = crate::edit::seed::load_or_seed(&paths).aspect;
        let same_aspect = same_folder && matches!(guard.as_ref(), Some(c) if c.aspect == aspect);
        if same_aspect {
            let c = guard.as_mut().unwrap();
            c.renderer.reload_edit(&paths);
            c.mtime = mtime;
        } else {
            let (renderer, meta) = build_renderer(&paths).map_err(|e| e.to_string())?;
            *guard = Some(Cached { folder: folder.to_string(), mtime, aspect, renderer, meta });
        }
    }
    f(guard.as_mut().unwrap(), &paths)
}

/// PNG-encode a BGRA buffer in-memory.
pub(crate) fn png_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut rgba = vec![0u8; bgra.len()];
    for i in (0..bgra.len()).step_by(4) {
        rgba[i] = bgra[i + 2];
        rgba[i + 1] = bgra[i + 1];
        rgba[i + 2] = bgra[i];
        rgba[i + 3] = bgra[i + 3];
    }
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().context("png write header")?;
    writer.write_image_data(&rgba).context("png write image data")?;
    drop(writer);
    Ok(out)
}

/// Tauri command: render one preview frame and return a PNG data URL. Reuses the warm
/// renderer cache (`with_warm`), so scrubbing is fast and edits still take effect.
#[tauri::command]
pub fn preview_frame(folder: String, time_ms: u32, session: tauri::State<'_, PreviewSession>) -> Result<String, String> {
    let png = with_warm(&session, &folder, |c, paths| {
        render_frame(&mut c.renderer, &c.meta, paths, time_ms).map_err(|e| e.to_string())
    })?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// Tauri command: the export background (BGRA mesh/gradient) as a PNG data URL, so the
/// editor's canvas preview paints the exact same background the export uses instead of an
/// approximate gradient. Reuses the warm renderer cache.
#[tauri::command]
pub fn preview_bg(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<String, String> {
    let png = with_warm(&session, &folder, |c, _paths| {
        png_encode(c.renderer.bg(), c.meta.out_w, c.meta.out_h).map_err(|e| e.to_string())
    })?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// Base64-encode bytes (RFC 4648, no padding line-breaks).
pub(crate) fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[(n >> 18) & 63] as char);
        out.push(CHARS[(n >> 12) & 63] as char);
        out.push(if chunk.len() > 1 { CHARS[(n >> 6) & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[n & 63] as char } else { '=' });
    }
    out
}

pub mod preprocess;
pub mod preview_fx;
pub mod preview_layouts;
pub mod preview_track;
pub mod thumbs;
