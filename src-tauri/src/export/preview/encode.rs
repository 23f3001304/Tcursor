//! Encoding a composited preview frame for the wire: PNG (lossless, for the background and the
//! bench) and JPEG (what the stage actually shows), plus the base64 the data URLs need. Pure
//! output formatting - no rendering, no decoding - split out of `preview/mod.rs` for its line
//! budget so that file stays the render path alone.
use anyhow::{Context, Result};

/// JPEG-encode a BGRA buffer through ffmpeg (`-q:v 3`, visually lossless for a preview). *Why
/// not `png_encode`:* the stage draws this frame the moment playback pauses or a scrub settles,
/// and a 1280-wide PNG deflate in a debug build costs more than the render itself; ffmpeg's
/// encoder is native code at any build profile and the result is a tenth of the bytes over IPC.
pub(crate) fn jpeg_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let tmp = crate::export::pipeline::ffio::StagedInput::new(bgra)?;
    let out = crate::win::sys::proc::ffcmd("ffmpeg")
        .args(["-v", "error", "-f", "rawvideo", "-pix_fmt", "bgra", "-video_size", &format!("{w}x{h}"), "-i"]).arg(tmp.path())
        .args(["-frames:v", "1", "-q:v", "3", "-f", "mjpeg", "-"])
        .stderr(std::process::Stdio::null())
        .output().context("spawn ffmpeg (jpeg encode)")?;
    if !out.status.success() || out.stdout.len() < 4 { anyhow::bail!("jpeg encode produced {} bytes", out.stdout.len()); }
    Ok(out.stdout)
}
#[cfg(test)] #[path = "jpeg_tests.rs"] mod jpeg_tests;

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
