// Render one composited frame at an arbitrary time T from edit.json.
// A WARM FrameRenderer (GPU pipeline, probed dims, decoded background, loaded
// events/edit/actions) is cached per recording in PreviewSession and reused across
// scrubs - only the per-frame seek-decode + composite + PNG-encode rerun. The cache is
// keyed by (folder, edit.json mtime, aspect), so an edit (which rewrites edit.json)
// transparently rebuilds it and the next preview reflects the change.
use anyhow::{Context, Result};
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::render::{FramePose, FrameRenderer, RenderMeta, OUT_FPS, OUT_STEP_MS};
use crate::export::settings::Resolution;
use crate::export::types::Layout;
use crate::session::paths::ProjectPaths;

/// The warm renderer cache lives in `session.rs` (line budget); re-exported so every preview
/// command keeps importing it from `crate::export::preview`.
pub use session::PreviewSession;
pub(crate) use session::with_warm;

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
/// Step the camera along the frame plan up to the output frame that shows `time_ms` (clip time):
/// the same `walk_plan` the exporter and `camera_track` run, so the one-shot frame is the export's.
/// With everything cut there is no plan; the camera is stepped once at output time 0 instead.
pub(crate) fn walk_to(r: &mut FrameRenderer, video_start: u64, time_ms: u32) -> FramePose {
    let map = r.time_map().clone();
    let plan = map.frame_plan(OUT_FPS);
    if plan.is_empty() { return r.step_camera(video_start, 0, OUT_STEP_MS); }
    let j_target = (map.out_of(time_ms) as u64 * OUT_FPS / 1000).min(plan.len() as u64 - 1) as usize;
    r.walk_plan(video_start, OUT_FPS, &plan, j_target, OUT_STEP_MS, |_, _, _, _| true).expect("plan is non-empty")
}

fn render_frame(renderer: &mut FrameRenderer, meta: &RenderMeta, paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>> {
    // step_camera requires ascending t; rewind so the cached renderer can re-scan to T.
    renderer.reset_camera();
    let pose = walk_to(renderer, meta.video_start, time_ms);

    // Seek-decode one screen frame at time_ms (the screen file's frame 0 is video_start).
    let mut screen_buf = vec![0u8; meta.screen_bytes];
    let mut screen_dec = RawDecoder::spawn(
        &paths.video(), 0.0, false, Some(time_ms as u64), None, None, "nv12", meta.screen_bytes)?;
    if !screen_dec.read_frame(&mut screen_buf)? {
        anyhow::bail!("no screen frame at {time_ms}ms (past end of video)");
    }
    drop(screen_dec);

    // Seek-decode one webcam frame if present (export pre-seeks webcam by video_start).
    let wc_dims = (meta.webcam_w, meta.webcam_h); // source-aspect decode box, matching the export
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;
    let webcam: Option<(Vec<u8>, u32, u32)> = if paths.webcam().exists() {
        let mut buf = vec![0u8; wc_bytes];
        let mut wc_dec = RawDecoder::spawn(
            &paths.webcam(), OUT_FPS as f64, false,
            Some(meta.video_start + time_ms as u64), Some(wc_dims), None, "bgra", wc_bytes)?;
        // A missing or unreadable webcam frame at THIS instant must not fail the whole preview:
        // `webcam.webm` routinely ends before `video.mp4`, so an `-ss` past its end is an everyday
        // scrub near the clip end. Both EOF and a hard decode failure leave `buf` untouched, which
        // is exactly what happened before failures became distinguishable from EOF. Only the
        // EXPORT (a deliverable) turns a decode failure into an error/warning.
        if let Err(e) = wc_dec.read_frame(&mut buf) { eprintln!("[PREVIEW] webcam frame at {time_ms}ms: {e}"); }
        drop(wc_dec);
        Some((buf, wc_dims.0, wc_dims.1))
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
///
/// `async` + `spawn_blocking` (Task 41 fix-up - see `mod.md` for the full why/how): `render_frame`
/// blocks on a real `ffmpeg` subprocess (`RawDecoder::spawn`), not in-process math. `State<'_,>`
/// isn't `'static`, so `app: AppHandle` is taken instead and re-derives the session inside the
/// blocking closure via `app.state::<PreviewSession>()`.
#[tauri::command]
pub async fn preview_frame(folder: String, time_ms: u32, app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let session = app.state::<PreviewSession>();
        let png = with_warm(&session, &folder, |c, paths| {
            render_frame(&mut c.renderer, &c.meta, paths, time_ms).map_err(|e| e.to_string())
        })?;
        Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tauri command: the export background (BGRA mesh/gradient) as a PNG data URL, so the
/// editor's canvas preview paints the exact same background the export uses instead of an
/// approximate gradient. Reuses the warm renderer cache.
///
/// `async` + `spawn_blocking`, same pattern (and same `AppHandle`-instead-of-`State` reason) as
/// `preview_frame` above: an earlier sweep left this sync on the grounds that a WARM call is only
/// a field read + PNG encode, but the COLD path runs `FrameRenderer::new` (ffprobe/ffmpeg
/// subprocesses + wgpu init) and the full-size PNG encode itself is a multi-MB swizzle + deflate
/// that every background-settings pointermove re-runs.
#[tauri::command]
pub async fn preview_bg(folder: String, app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let session = app.state::<PreviewSession>();
        let png = with_warm(&session, &folder, |c, _paths| {
            png_encode(c.renderer.bg(), c.meta.out_w, c.meta.out_h).map_err(|e| e.to_string())
        })?;
        Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
    })
    .await
    .map_err(|e| e.to_string())?
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

pub mod bg_thumbs; pub mod preprocess; pub mod preview_fx; pub mod preview_layouts; pub mod preview_track; pub mod session; pub mod thumbs;
