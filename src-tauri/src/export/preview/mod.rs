// Render one composited frame at an arbitrary time T from edit.json.
// A WARM FrameRenderer (GPU pipeline, probed dims, decoded background, loaded
// events/edit/actions) is cached per recording in PreviewSession and reused across
// scrubs - only the per-frame seek-decode + composite + PNG-encode rerun. The cache is
// keyed by (folder, edit.json mtime, aspect), so an edit (which rewrites edit.json)
// transparently rebuilds it and the next preview reflects the change.
use anyhow::Result;
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::render::{FramePose, FrameRenderer, RenderMeta, OUT_FPS, OUT_STEP_MS};
use crate::export::settings::Resolution;
use crate::export::types::Layout;
use crate::session::paths::ProjectPaths;
// PNG/JPEG/base64 encoding of a composited frame - the wire format, not the render; split out
// of this file for its line budget (`encode.rs`).
pub(crate) use encode::{base64_encode, jpeg_encode, png_encode};

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

/// Seek-decode one screen frame at `time_ms` (clip time; the screen file's frame 0 is
/// `video_start`) into `buf`, as nv12 at the renderer's own crop.
fn decode_screen(paths: &ProjectPaths, meta: &RenderMeta, time_ms: u32, buf: &mut [u8]) -> Result<()> {
    let mut dec = RawDecoder::spawn(
        &paths.video(), 0.0, false, Some(time_ms as u64), meta.screen_crop, None, None, "nv12", meta.screen_bytes)?;
    if !dec.read_frame(buf)? { anyhow::bail!("no screen frame at {time_ms}ms (past end of video)"); }
    Ok(())
}

/// The composited BGRA frame at `time_ms` (`meta.out_w` x `meta.out_h`); the callers encode it.
fn composite_frame(renderer: &mut FrameRenderer, meta: &RenderMeta, paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>> {
    // step_camera requires ascending t; rewind so the cached renderer can re-scan to T.
    renderer.reset_camera();
    let pose = walk_to(renderer, meta.video_start, time_ms);

    let mut screen_buf = vec![0u8; meta.screen_bytes];
    decode_screen(paths, meta, time_ms, &mut screen_buf)?;

    // Inside a mid-take display switch, the dissolve blends the frame the EXPORT latches - the
    // last output frame before the switch (`SpanMix::hold_ms`) - so the ghost image here is the
    // export's own. A decode failure (the instant fell outside the clip) just drops the dissolve
    // rather than failing a scrub; only the export treats a screen decode as a deliverable.
    let prev = pose.mix.and_then(|m| {
        let clip = renderer.time_map().clip_of(m.hold_ms);
        let mut b = vec![0u8; meta.screen_bytes];
        decode_screen(paths, meta, clip, &mut b).ok().map(|()| b)
    });

    // Seek-decode one webcam frame if present (export pre-seeks webcam by video_start).
    let wc_dims = (meta.webcam_w, meta.webcam_h); // source-aspect decode box, matching the export
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;
    let webcam: Option<(Vec<u8>, u32, u32)> = if paths.webcam().exists() {
        let mut buf = vec![0u8; wc_bytes];
        let mut wc_dec = RawDecoder::spawn(
            &paths.webcam(), OUT_FPS as f64, false,
            Some(meta.video_start + time_ms as u64), None, Some(wc_dims), None, "bgra", wc_bytes)?;
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
    renderer.composite_at(&pose, &screen_buf, prev.as_deref(), wc_ref, &mut bgra);
    Ok(bgra)
}

/// Render one composited frame at `time_ms` into the recording at `paths` (uncached:
/// builds a fresh renderer). Returns PNG bytes at the resolved preview size.
pub fn render_preview(paths: &ProjectPaths, time_ms: u32) -> Result<Vec<u8>> {
    let (mut renderer, meta) = build_renderer(paths)?;
    let bgra = composite_frame(&mut renderer, &meta, paths, time_ms)?;
    png_encode(&bgra, meta.out_w, meta.out_h)
}

/// Tauri command: render one preview frame and return a JPEG data URL - the export's own frame
/// at that instant, which the stage shows whenever playback pauses or a scrub settles so what the
/// owner looks at IS the export (owner ruling 2026-09-14: the export is the reference). Reuses the
/// warm renderer cache (`with_warm`), so scrubbing is fast and edits still take effect.
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
        let jpeg = with_warm(&session, &folder, |c, paths| {
            let bgra = composite_frame(&mut c.renderer, &c.meta, paths, time_ms).map_err(|e| e.to_string())?;
            jpeg_encode(&bgra, c.meta.out_w, c.meta.out_h).map_err(|e| e.to_string())
        })?;
        Ok(format!("data:image/jpeg;base64,{}", base64_encode(&jpeg)))
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

pub mod bg_thumbs; pub mod encode; pub mod preprocess; pub mod preview_fx; pub mod preview_layouts; pub mod preview_track; pub mod segments_audio; pub mod segments_webcam; pub mod session; pub mod thumbs;
