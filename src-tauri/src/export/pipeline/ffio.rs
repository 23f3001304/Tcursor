// ffmpeg/ffprobe spawn, frame-read, and bundled-image decode/crop helpers.
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Stdio;
use crate::win::sys::proc::ffcmd;

#[path = "ffio_decoder.rs"]
mod ffio_decoder;
pub use ffio_decoder::RawDecoder;

/// Probe a video's pixel dimensions via ffprobe (`width,height`).
pub fn probe_dims(video: &Path) -> Result<(u32, u32)> {
    let out = ffcmd("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0",
            "-show_entries", "stream=width,height", "-of", "csv=p=0"])
        .arg(video)
        .stderr(Stdio::null())
        .output()
        .context("spawn ffprobe (dims)")?;
    let s = String::from_utf8_lossy(&out.stdout);
    let line = s.trim().lines().next().unwrap_or("").trim();
    let mut it = line.split(',');
    let w = it.next().and_then(|v| v.trim().parse().ok());
    let h = it.next().and_then(|v| v.trim().parse().ok());
    match (w, h) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(anyhow!("ffprobe dims parse failed: {line:?}")),
    }
}

/// Probe a video's duration in seconds via ffprobe (0.0 if unavailable).
pub fn probe_duration(video: &Path) -> Result<f64> {
    let out = ffcmd("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration",
            "-of", "default=nk=1:nw=1"])
        .arg(video)
        .stderr(Stdio::null())
        .output()
        .context("spawn ffprobe (duration)")?;
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.trim().lines().next().unwrap_or("0").trim().parse().unwrap_or(0.0))
}

/// An image staged on disk for ffmpeg to read, at a path NO other call can pick, deleted when
/// the guard drops (including on an early `?` return, which the old inline cleanup skipped).
///
/// Why a per-call path: this used to write every image to one fixed `$TEMP/cursorzoom_bg_src`,
/// and `decode_image` has two unrelated callers that run on different threads - `background::build`
/// (the mesh wallpaper, on a cold `FrameRenderer::new` for a preview OR an export) and
/// `decode_cursor` (every cursor-pack sprite, via the `cursor_sprites` command and `cursorset::prep`).
/// The editor fires both within milliseconds of opening a project, so a sprite decode routinely
/// overwrote the wallpaper's staged bytes in the window between the write and ffmpeg's read: the
/// background came back as `resize_ns.png` stretched to the full frame (what `preview_bg` then
/// returned, and what `composite_at` drew under every panel).
struct StagedInput(std::path::PathBuf);

impl StagedInput {
    /// Write `bytes` to a fresh `$TEMP/cursorzoom_img_<pid>_<n>`: unique within the process by the
    /// counter, across processes by the pid, so concurrent decodes can never share an input file.
    fn new(bytes: &[u8]) -> Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cursorzoom_img_{}_{n}", std::process::id()));
        std::fs::write(&path, bytes).context("write image temp")?;
        Ok(Self(path))
    }
    fn path(&self) -> &Path { &self.0 }
}

impl Drop for StagedInput {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

/// Decode `image` bytes (any ffmpeg-readable format) to a `w*h*4` BGRA buffer,
/// STRETCHED to the output size. Used for the legacy bundled background (`bg.jpg`), whose look
/// every pre-existing project depends on - hence a plain `scale`, aspect ratio and all.
pub fn decode_image(image: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    decode_with(image, w, h, format!("scale={w}:{h}"))
}

/// `decode_image` with a COVER fit: the image is scaled up until it fills `w`x`h` with its aspect
/// ratio intact, then centre-cropped. Used for the bundled wallpaper library
/// (`settings::wallpapers`), whose 16:9 art would visibly stretch in a 9:16 or 1:1 export.
pub fn decode_image_cover(image: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    decode_with(image, w, h, format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"))
}

/// `decode_image_cover` for a file already ON DISK - the user's imported background
/// (`settings::bg_asset`), whose bytes must not be read into memory and re-staged: a still can be
/// tens of MB and a video background is decoded here too (for its FIRST frame, which is what the
/// preview commands show and what the export falls back to if its stream dies).
pub fn decode_file_cover(file: &Path, w: u32, h: u32) -> Result<Vec<u8>> {
    decode_from(file, w, h, format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"))
}

fn decode_with(image: &[u8], w: u32, h: u32, vf: String) -> Result<Vec<u8>> {
    let tmp = StagedInput::new(image)?;
    decode_from(tmp.path(), w, h, vf)
}

fn decode_from(file: &Path, w: u32, h: u32, vf: String) -> Result<Vec<u8>> {
    let out = ffcmd("ffmpeg")
        .args(["-v", "error", "-i"]).arg(file)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "bgra", "-vf", &vf, "-"])
        .stderr(Stdio::null())
        .output()
        .context("spawn ffmpeg (image decode)")?;
    if out.stdout.len() == (w * h * 4) as usize {
        Ok(out.stdout)
    } else {
        Err(anyhow!("image decode produced {} bytes", out.stdout.len()))
    }
}

/// Read a PNG's pixel dimensions from its IHDR chunk (width @ byte 16, height @ 20,
/// big-endian). Returns None if the bytes are not a PNG or are truncated.
pub fn png_dims(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || png[0..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        return None;
    }
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    if w > 0 && h > 0 { Some((w, h)) } else { None }
}

/// Trim transparent border rows/columns from a BGRA image (alpha <= 16 = empty). Returns the
/// cropped buffer, its (w, h), and the (left, top) origin of the crop. None if fully transparent.
pub fn crop_to_alpha(bgra: &[u8], w: u32, h: u32) -> Option<(Vec<u8>, u32, u32, u32, u32)> {
    if w == 0 || h == 0 { return None; }
    debug_assert!(bgra.len() >= w as usize * h as usize * 4);
    let (mut l, mut t, mut r, mut b) = (w, h, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            if bgra[((y * w + x) * 4 + 3) as usize] > 16 {
                l = l.min(x); r = r.max(x);
                t = t.min(y); b = b.max(y);
            }
        }
    }
    if r < l || b < t { return None; }
    let (cw, ch) = (r - l + 1, b - t + 1);
    let row = (cw * 4) as usize;
    let mut out = vec![0u8; row * ch as usize];
    for y in 0..ch {
        let s = (((t + y) * w + l) * 4) as usize;
        let d = (y * cw * 4) as usize;
        out[d..d + row].copy_from_slice(&bgra[s..s + row]);
    }
    Some((out, cw, ch, l, t))
}

/// Decode a cursor PNG to a content-tight BGRA sprite; `hot` (canvas fraction) is re-based to
/// the content box. Returns (bgra, content_w, content_h, content_hotspot, native_canvas_h);
/// the canvas height lets a pack scale all cursors uniformly (keeping relative sizes).
pub fn decode_cursor(png: &[u8], hot: (f32, f32)) -> Option<(Vec<u8>, u32, u32, (f32, f32), u32)> {
    let (nw, nh) = png_dims(png)?;
    let src = decode_image(png, nw, nh).ok()?;
    let (out, w, h, l, t) = crop_to_alpha(&src, nw, nh)?;
    let hx = (hot.0 * nw as f32 - l as f32) / w as f32;
    let hy = (hot.1 * nh as f32 - t as f32) / h as f32;
    Some((out, w, h, (hx, hy), nh))
}

/// Number of video frames (via ffprobe `nb_frames`, else `avg_frame_rate`*duration).
pub fn probe_frame_count(video: &Path) -> Result<u64> {
    let out = ffcmd("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0",
            "-show_entries", "stream=nb_frames,avg_frame_rate,duration", "-of", "default=nw=1"])
        .arg(video)
        .stderr(Stdio::null())
        .output()
        .context("spawn ffprobe (frames)")?;
    let s = String::from_utf8_lossy(&out.stdout);
    let (mut nb, mut rate, mut dur) = (None, None, None);
    for line in s.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("nb_frames=") { nb = v.parse::<u64>().ok().filter(|n| *n > 0); }
        else if let Some(v) = line.strip_prefix("avg_frame_rate=") {
            let mut it = v.split('/');
            let a = it.next().and_then(|x| x.parse::<f64>().ok());
            let b = it.next().and_then(|x| x.parse::<f64>().ok());
            if let (Some(a), Some(b)) = (a, b) { if b > 0.0 { rate = Some(a / b); } }
        } else if let Some(v) = line.strip_prefix("duration=") { dur = v.parse::<f64>().ok(); }
    }
    if let Some(n) = nb { return Ok(n); }
    if let (Some(r), Some(d)) = (rate, dur) { return Ok((r * d).round() as u64); }
    Err(anyhow!("could not determine frame count"))
}

#[cfg(test)]
#[path = "ffio_tests.rs"]
mod tests;
