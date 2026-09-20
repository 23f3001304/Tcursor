use crate::export::render::{FrameRenderer, RenderMeta};
use crate::export::settings::Resolution;
use crate::export::types::Layout;
use crate::ports::system::SystemPort;
use crate::session::paths::ProjectPaths;
use anyhow::{Context, Result};

use compose::composite_frame;
pub use compose::{at_instants, PreviewAt};
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

pub fn render_preview(
    paths: &ProjectPaths,
    time_ms: u32,
    system: &dyn SystemPort,
) -> Result<Vec<u8>> {
    let (mut renderer, meta) = build_renderer(paths, system)?;
    let bgra = composite_frame(&mut renderer, &meta, paths, PreviewAt::Clip(time_ms))?;
    png_encode(&bgra, meta.out_w, meta.out_h)
}

#[tauri::command]
pub async fn preview_frame(
    folder: String,
    out_ms: u32,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let jpeg = with_warm_app(&app, &folder, |c, paths| {
            let bgra = composite_frame(&mut c.renderer, &c.meta, paths, PreviewAt::Out(out_ms))
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
pub mod compose;
pub mod preprocess;
pub mod preview_fx;
pub mod preview_layouts;
pub mod preview_track;
pub mod segments_audio;
pub mod segments_webcam;
pub mod session;
pub mod thumbs;
