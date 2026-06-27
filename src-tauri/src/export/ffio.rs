// ffmpeg/ffprobe spawn + frame-read helpers for the exporter pipeline.
use anyhow::{anyhow, Context, Result};
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Stdio};
use crate::win::proc::ffcmd;

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

/// Decode `image` bytes (any ffmpeg-readable format) to a `w*h*4` BGRA buffer,
/// scaled to the output size. Used for the bundled background wallpaper.
pub fn decode_image(image: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let tmp = std::env::temp_dir().join("cursorzoom_bg_src");
    std::fs::write(&tmp, image).context("write bg temp")?;
    let out = ffcmd("ffmpeg")
        .args(["-v", "error", "-i"]).arg(&tmp)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "bgra",
            "-vf", &format!("scale={w}:{h}"), "-"])
        .stderr(Stdio::null())
        .output()
        .context("spawn ffmpeg (bg decode)")?;
    let _ = std::fs::remove_file(&tmp);
    if out.stdout.len() == (w * h * 4) as usize {
        Ok(out.stdout)
    } else {
        Err(anyhow!("bg decode produced {} bytes", out.stdout.len()))
    }
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

/// A spawned ffmpeg decoder emitting raw BGRA frames of a fixed byte size.
pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
}

impl RawDecoder {
    /// Spawn `ffmpeg` decoding `video` to rawvideo BGRA. `rate <= 0` decodes at
    /// the native frame rate (used when the caller places frames by timestamp);
    /// otherwise `rate` is applied as an input (`input_rate`) or output `-r`.
    /// `seek_ms` trims the start (`-ss`); `scale` cover-crops to a square.
    pub fn spawn(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, scale: Option<u32>, frame_bytes: usize) -> Result<Self> {
        let r = format!("{rate:.4}");
        let mut cmd = ffcmd("ffmpeg");
        cmd.args(["-v", "error"]);
        if let Some(ms) = seek_ms { cmd.args(["-ss", &format!("{:.4}", ms as f64 / 1000.0)]); }
        if rate > 0.0 && input_rate { cmd.args(["-r", &r]); }
        cmd.arg("-i").arg(video).args(["-f", "rawvideo", "-pix_fmt", "bgra"]);
        if rate > 0.0 && !input_rate { cmd.args(["-r", &r]); }
        if let Some(sz) = scale {
            // Cover-crop to a centered square so a 16:9 webcam isn't skewed.
            cmd.args(["-vf", &format!("scale={sz}:{sz}:force_original_aspect_ratio=increase,crop={sz}:{sz}")]);
        }
        let mut child = cmd.arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn ffmpeg decoder")?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("decoder stdout missing"))?;
        Ok(Self { child, stdout, frame_bytes })
    }

    /// Read exactly one frame into `buf`. Returns Ok(false) at end-of-stream.
    pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool> {
        debug_assert_eq!(buf.len(), self.frame_bytes);
        match self.stdout.read_exact(buf) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(anyhow!("decoder read failed: {e}")),
        }
    }
}

impl Drop for RawDecoder {
    fn drop(&mut self) {
        // Drop the read pipe so ffmpeg sees a broken pipe and exits, then reap.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
