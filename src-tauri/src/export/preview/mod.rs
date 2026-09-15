use crate::export::pipeline::ffio::RawDecoder;
use crate::export::render::{FramePose, FrameRenderer, RenderMeta, OUT_FPS, OUT_STEP_MS};
use crate::export::settings::Resolution;
use crate::export::types::Layout;
use crate::ports::system::SystemPort;
use crate::session::paths::ProjectPaths;
use anyhow::{Context, Result};

pub(crate) use session::with_warm_app;
pub use session::PreviewSession;

const PREVIEW_LONG_EDGE: u32 = 1280;

fn build_renderer(
    paths: &ProjectPaths,
    system: &dyn SystemPort,
) -> Result<(FrameRenderer, RenderMeta)> {
    let fps = system.primary_refresh_hz().min(60);
    FrameRenderer::new(
        paths,
        Layout::default(),
        fps,
        Resolution::Source,
        Some(PREVIEW_LONG_EDGE),
        system,
    )
}

pub(crate) fn walk_to(r: &mut FrameRenderer, video_start: u64, time_ms: u32) -> FramePose {
    let map = r.time_map().clone();
    let plan = map.frame_plan(OUT_FPS);
    if plan.is_empty() {
        return r.step_camera(video_start, 0, OUT_STEP_MS);
    }
    let j_target =
        (map.out_of(time_ms) as u64 * OUT_FPS / 1000).min(plan.len() as u64 - 1) as usize;
    r.walk_plan(
        video_start,
        OUT_FPS,
        &plan,
        j_target,
        OUT_STEP_MS,
        |_, _, _, _| true,
    )
    .expect("plan is non-empty")
}

fn decode_screen(
    paths: &ProjectPaths,
    meta: &RenderMeta,
    time_ms: u32,
    buf: &mut [u8],
) -> Result<()> {
    let mut dec = RawDecoder::spawn(
        &paths.video(),
        0.0,
        false,
        Some(time_ms as u64),
        meta.screen_crop,
        None,
        None,
        "nv12",
        meta.screen_bytes,
    )?;
    if !dec.read_frame(buf)? {
        anyhow::bail!("no screen frame at {time_ms}ms (past end of video)");
    }
    Ok(())
}

fn composite_frame(
    renderer: &mut FrameRenderer,
    meta: &RenderMeta,
    paths: &ProjectPaths,
    time_ms: u32,
) -> Result<Vec<u8>> {
    renderer.reset_camera();
    let pose = walk_to(renderer, meta.video_start, time_ms);

    let mut screen_buf = vec![0u8; meta.screen_bytes];
    decode_screen(paths, meta, time_ms, &mut screen_buf)?;

    let prev = pose.mix.and_then(|m| {
        let clip = renderer.time_map().clip_of(m.hold_ms);
        let mut b = vec![0u8; meta.screen_bytes];
        decode_screen(paths, meta, clip, &mut b).ok().map(|()| b)
    });

    let wc_dims = (meta.webcam_w, meta.webcam_h);
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;
    let webcam: Option<(Vec<u8>, u32, u32)> = if paths.webcam().exists() {
        let mut buf = vec![0u8; wc_bytes];
        let mut wc_dec = RawDecoder::spawn(
            &paths.webcam(),
            OUT_FPS as f64,
            false,
            Some(meta.video_start + time_ms as u64),
            None,
            Some(wc_dims),
            None,
            "bgra",
            wc_bytes,
        )?;
        if let Err(e) = wc_dec.read_frame(&mut buf) {
            eprintln!("[PREVIEW] webcam frame at {time_ms}ms: {e}");
        }
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

pub fn render_preview(
    paths: &ProjectPaths,
    time_ms: u32,
    system: &dyn SystemPort,
) -> Result<Vec<u8>> {
    let (mut renderer, meta) = build_renderer(paths, system)?;
    let bgra = composite_frame(&mut renderer, &meta, paths, time_ms)?;
    png_encode(&bgra, meta.out_w, meta.out_h)
}

#[tauri::command]
pub async fn preview_frame(
    folder: String,
    time_ms: u32,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let jpeg = with_warm_app(&app, &folder, |c, paths| {
            let bgra = composite_frame(&mut c.renderer, &c.meta, paths, time_ms)
                .map_err(|e| e.to_string())?;
            jpeg_encode(&bgra, c.meta.out_w, c.meta.out_h).map_err(|e| e.to_string())
        })?;
        Ok(format!("data:image/jpeg;base64,{}", base64_encode(&jpeg)))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn preview_bg(folder: String, app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let png = with_warm_app(&app, &folder, |c, _paths| {
            png_encode(c.renderer.bg(), c.meta.out_w, c.meta.out_h).map_err(|e| e.to_string())
        })?;
        Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub(crate) const JPEG_QUALITY: u8 = 90;

pub(crate) fn jpeg_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let (w16, h16) = (u16::try_from(w)?, u16::try_from(h)?);
    let mut out = Vec::with_capacity(bgra.len() / 8);
    jpeg_encoder::Encoder::new(&mut out, JPEG_QUALITY)
        .encode(bgra, w16, h16, jpeg_encoder::ColorType::Bgra)
        .context("jpeg encode")?;
    if out.len() < 4 {
        anyhow::bail!("jpeg encode produced {} bytes", out.len());
    }
    Ok(out)
}

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
    writer
        .write_image_data(&rgba)
        .context("png write image data")?;
    drop(writer);
    Ok(out)
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[(n >> 18) & 63] as char);
        out.push(CHARS[(n >> 12) & 63] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(n >> 6) & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[n & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

pub mod bg_thumbs;
pub mod preprocess;
pub mod preview_fx;
pub mod preview_layouts;
pub mod preview_track;
pub mod segments_audio;
pub mod segments_webcam;
pub mod session;
pub mod thumbs;
