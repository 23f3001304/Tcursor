// ffmpeg/ffprobe spawn + frame-read helpers for the exporter pipeline.
use anyhow::{anyhow, Context, Result};
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

/// Probe a video's pixel dimensions via ffprobe (`width,height`).
pub fn probe_dims(video: &Path) -> Result<(u32, u32)> {
    let out = Command::new("ffprobe")
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
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration",
            "-of", "default=nk=1:nw=1"])
        .arg(video)
        .stderr(Stdio::null())
        .output()
        .context("spawn ffprobe (duration)")?;
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.trim().lines().next().unwrap_or("0").trim().parse().unwrap_or(0.0))
}

/// A spawned ffmpeg decoder emitting raw BGRA frames of a fixed byte size.
pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
}

impl RawDecoder {
    /// Spawn `ffmpeg` decoding `video` to rawvideo BGRA at `fps`, optionally
    /// scaled to `scale x scale`. `frame_bytes` is the expected bytes per frame.
    pub fn spawn(video: &Path, fps: u32, scale: Option<u32>, frame_bytes: usize) -> Result<Self> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-i"]).arg(video)
            .args(["-f", "rawvideo", "-pix_fmt", "bgra", "-r", &fps.to_string()]);
        if let Some(sz) = scale {
            cmd.args(["-vf", &format!("scale={sz}:{sz}")]);
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
