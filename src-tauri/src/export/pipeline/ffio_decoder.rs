// `RawDecoder`: a spawned ffmpeg process emitting raw BGRA frames, split out of
// ffio.rs (which keeps the ffprobe/decode/crop free-function helpers) so ffio.rs
// stays under the size limit. Re-exported by ffio.rs as `ffio::RawDecoder`, so
// every existing import path (`crate::export::pipeline::ffio::RawDecoder`) still
// resolves unchanged.
use anyhow::{anyhow, Context, Result};
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Stdio};
use crate::win::sys::proc::ffcmd;

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
    /// `seek_ms` trims the start (`-ss`); `square_scale` cover-crops to a square;
    /// `target_dims` scales output dimensions directly in FFmpeg.
    pub fn spawn(
        video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>,
        square_scale: Option<u32>, target_dims: Option<(u32, u32)>, frame_bytes: usize
    ) -> Result<Self> {
        let r = format!("{rate:.4}");
        let mut cmd = ffcmd("ffmpeg");
        cmd.args(["-v", "error", "-hwaccel", "auto"]);
        if let Some(ms) = seek_ms { cmd.args(["-ss", &format!("{:.4}", ms as f64 / 1000.0)]); }
        if rate > 0.0 && input_rate { cmd.args(["-r", &r]); }
        cmd.arg("-i").arg(video);
        if rate > 0.0 && !input_rate { cmd.args(["-r", &r]); }
        cmd.args(["-sws_flags", "fast_bilinear"]);
        if let Some((w, h)) = target_dims {
            cmd.args(["-vf", &format!("scale={w}:{h}:flags=fast_bilinear")]);
        } else if let Some(sz) = square_scale {
            // Cover-crop to a centered square so a 16:9 webcam isn't skewed.
            cmd.args(["-vf", &format!("scale={sz}:{sz}:force_original_aspect_ratio=increase,crop={sz}:{sz}:flags=fast_bilinear")]);
        }
        cmd.args(["-f", "rawvideo", "-pix_fmt", "bgra"]);
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
