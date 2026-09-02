// `RawDecoder`: a spawned ffmpeg process emitting raw BGRA frames, split out of
// ffio.rs (which keeps the ffprobe/decode/crop free-function helpers) so ffio.rs
// stays under the size limit. Re-exported by ffio.rs as `ffio::RawDecoder`, so
// every existing import path (`crate::export::pipeline::ffio::RawDecoder`) still
// resolves unchanged.
use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use crate::win::sys::proc::ffcmd;

/// Retained ffmpeg stderr (bytes). Enough for a filtergraph rejection or a decoder error;
/// once full the whole buffer is dropped rather than sliced (never splits a char boundary).
const TAIL_CAP: usize = 2000;

/// A spawned ffmpeg decoder emitting raw BGRA frames of a fixed byte size.
pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
    stderr: Arc<Mutex<String>>,
    drain: Option<JoinHandle<()>>,
    frames: u64,
}

/// Build the `ffmpeg` CLI args for `RawDecoder::spawn`, pure (no process spawn) so the
/// `-r`/`-vf`/`-pix_fmt` selection is unit-testable without launching ffmpeg. Mirrors the
/// exact arg order `spawn` used to build inline via `Command::args`.
fn decode_args(
    video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>,
    cover_scale: Option<(u32, u32)>, target_dims: Option<(u32, u32)>, pix_fmt: &str,
) -> Vec<String> {
    let r = format!("{rate:.4}");
    let mut args: Vec<String> = ["-v", "error", "-hwaccel", "auto"].map(String::from).into();
    if let Some(ms) = seek_ms { args.push("-ss".into()); args.push(format!("{:.4}", ms as f64 / 1000.0)); }
    if rate > 0.0 && input_rate { args.push("-r".into()); args.push(r.clone()); }
    args.push("-i".into());
    args.push(video.to_string_lossy().into_owned());
    if rate > 0.0 && !input_rate { args.push("-r".into()); args.push(r); }
    args.push("-sws_flags".into()); args.push("fast_bilinear".into());
    if let Some((w, h)) = target_dims {
        args.push("-vf".into()); args.push(format!("scale={w}:{h}:flags=fast_bilinear"));
    } else if let Some((w, h)) = cover_scale {
        // Cover-crop to a centered `w`x`h`. For the webcam that box carries the SOURCE's own
        // aspect (`render::meta::webcam_box`), so this is effectively a scale-to-fit and the
        // full frame survives for the compositors to crop per panel; forcing any one panel's
        // aspect here threw away pixels a differently-shaped panel later needed.
        // NOTE: `flags` is a `scale` option, NOT a `crop` option - putting it on `crop` makes
        // newer ffmpeg reject the whole filtergraph ("Option not found"), which silently zeroed
        // the webcam decode and dropped the camera from every export. The global `-sws_flags
        // fast_bilinear` above already sets the scale flags, so `crop` needs none.
        args.push("-vf".into());
        args.push(format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"));
    }
    // `nv12` (Y + interleaved half-res UV, ~2.6x smaller than bgra) is nvdec's native output, so
    // the screen decode skips the expensive yuv->bgra swscale AND pushes far fewer bytes through
    // the pipe (the export bottleneck); the GPU/CPU compositor does the color convert. The webcam
    // stays `bgra` (small, and its square cover-crop scale filter wants a packed format).
    args.push("-f".into()); args.push("rawvideo".into());
    args.push("-pix_fmt".into()); args.push(pix_fmt.to_string());
    args.push("-".into());
    args
}

/// Drain ffmpeg's stderr into a rolling tail on its own thread. A THREAD, not a read after
/// EOF: ffmpeg blocks once the ~64 KB stderr pipe fills and then stops writing stdout too, so
/// a reader waiting on the next frame would deadlock against the very message it is waiting for.
fn drain_stderr(pipe: ChildStderr, sink: Arc<Mutex<String>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            let mut t = sink.lock().unwrap_or_else(|e| e.into_inner());
            if t.len() + line.len() > TAIL_CAP { t.clear(); }
            t.push_str(&line);
            t.push('\n');
        }
    })
}

/// What a closed decoder stdout means. ffmpeg closes the pipe identically whether it finished
/// the file or died on it (unreadable/corrupt input, a rejected filtergraph, a missing codec),
/// so the EXIT STATUS decides: `Ok(false)` is a clean end of stream; a failure is an `Err`
/// carrying ffmpeg's own stderr tail, so the export aborts instead of silently compositing a
/// whole file of black frames over a decode that never produced anything.
fn classify_end(success: bool, status: &str, frames: u64, tail: &str) -> Result<bool> {
    if success { return Ok(false); }
    let why = if tail.is_empty() { "no stderr output" } else { tail };
    Err(anyhow!("ffmpeg decode failed ({status}) after {frames} frame(s): {why}"))
}

impl RawDecoder {
    /// Spawn `ffmpeg` decoding `video` to rawvideo BGRA. `rate <= 0` decodes at
    /// the native frame rate (used when the caller places frames by timestamp);
    /// otherwise `rate` is applied as an input (`input_rate`) or output `-r`.
    /// `seek_ms` trims the start (`-ss`); `cover_scale` cover-crops to a `(w, h)` box;
    /// `target_dims` scales output dimensions directly in FFmpeg.
    pub fn spawn(
        video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>,
        cover_scale: Option<(u32, u32)>, target_dims: Option<(u32, u32)>, pix_fmt: &str, frame_bytes: usize
    ) -> Result<Self> {
        let args = decode_args(video, rate, input_rate, seek_ms, cover_scale, target_dims, pix_fmt);
        let mut child = ffcmd("ffmpeg")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // captured, not discarded: it is the only account of WHY a decode died
            .spawn()
            .context("spawn ffmpeg decoder")?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("decoder stdout missing"))?;
        let stderr = Arc::new(Mutex::new(String::new()));
        let drain = child.stderr.take().map(|p| drain_stderr(p, stderr.clone()));
        Ok(Self { child, stdout, frame_bytes, stderr, drain, frames: 0 })
    }

    /// Read exactly one frame into `buf`. `Ok(false)` is a CLEAN end-of-stream only - a failed
    /// decode returns `Err` (see `classify_end`), never a silently-short stream.
    pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool> {
        debug_assert_eq!(buf.len(), self.frame_bytes);
        match self.stdout.read_exact(buf) {
            Ok(()) => { self.frames += 1; Ok(true) }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => self.end_of_stream(),
            Err(e) => Err(anyhow!("decoder read failed: {e}")),
        }
    }

    /// Reap the process and classify the stream end. Joins the stderr drain first - the pipe
    /// hits EOF the moment ffmpeg exits, so this returns at once and the tail is complete
    /// rather than racing the exit.
    fn end_of_stream(&mut self) -> Result<bool> {
        let st = self.child.wait().context("wait ffmpeg decoder")?;
        if let Some(h) = self.drain.take() { let _ = h.join(); }
        let tail = self.stderr.lock().unwrap_or_else(|e| e.into_inner()).trim().to_string();
        classify_end(st.success(), &st.to_string(), self.frames, &tail)
    }
}

impl Drop for RawDecoder {
    fn drop(&mut self) {
        // Drop the read pipe so ffmpeg sees a broken pipe and exits, then reap.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "ffio_decoder_tests.rs"]
mod tests;
