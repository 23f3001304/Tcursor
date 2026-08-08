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

/// Build the `ffmpeg` CLI args for `RawDecoder::spawn`, pure (no process spawn) so the
/// `-r`/`-vf`/`-pix_fmt` selection is unit-testable without launching ffmpeg. Mirrors the
/// exact arg order `spawn` used to build inline via `Command::args`.
fn decode_args(
    video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>,
    square_scale: Option<u32>, target_dims: Option<(u32, u32)>, pix_fmt: &str,
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
    } else if let Some(sz) = square_scale {
        // Cover-crop to a centered square so a 16:9 webcam isn't skewed. NOTE: `flags` is a
        // `scale` option, NOT a `crop` option - putting it on `crop` makes newer ffmpeg reject
        // the whole filtergraph ("Option not found"), which silently zeroed the webcam decode
        // and dropped the camera from every export. The global `-sws_flags fast_bilinear` above
        // already sets the scale flags, so `crop` needs none.
        args.push("-vf".into());
        args.push(format!("scale={sz}:{sz}:force_original_aspect_ratio=increase,crop={sz}:{sz}"));
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

impl RawDecoder {
    /// Spawn `ffmpeg` decoding `video` to rawvideo BGRA. `rate <= 0` decodes at
    /// the native frame rate (used when the caller places frames by timestamp);
    /// otherwise `rate` is applied as an input (`input_rate`) or output `-r`.
    /// `seek_ms` trims the start (`-ss`); `square_scale` cover-crops to a square;
    /// `target_dims` scales output dimensions directly in FFmpeg.
    pub fn spawn(
        video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>,
        square_scale: Option<u32>, target_dims: Option<(u32, u32)>, pix_fmt: &str, frame_bytes: usize
    ) -> Result<Self> {
        let args = decode_args(video, rate, input_rate, seek_ms, square_scale, target_dims, pix_fmt);
        let mut child = ffcmd("ffmpeg")
            .args(&args)
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

#[cfg(test)]
mod tests {
    use super::decode_args;
    use std::path::Path;

    /// The screen decode's post-fix call shape (`rate = out_fps as f64`, `input_rate =
    /// false`): the output `-r <out_fps>` must be present, placed AFTER `-i`, so the decode
    /// is rate-converted to the export fps instead of running at the source's native rate.
    #[test]
    fn positive_output_rate_emits_r_after_i() {
        let args = decode_args(Path::new("v.mp4"), 30.0, false, None, None, None, "nv12");
        let i = args.iter().position(|a| a == "-i").unwrap();
        let r = args.iter().position(|a| a == "-r").unwrap();
        assert!(r > i, "-r must come after -i for an output rate: {args:?}");
        assert_eq!(args[r + 1], "30.0000");
    }

    /// `input_rate = true` places `-r` BEFORE `-i` instead (the webcam's alternate mode).
    #[test]
    fn positive_input_rate_emits_r_before_i() {
        let args = decode_args(Path::new("v.mp4"), 60.0, true, None, None, None, "bgra");
        let i = args.iter().position(|a| a == "-i").unwrap();
        let r = args.iter().position(|a| a == "-r").unwrap();
        assert!(r < i, "-r must come before -i for an input rate: {args:?}");
    }

    /// `rate <= 0` decodes at the source's native rate: no `-r` at all (the pre-fix screen
    /// behavior, still used by callers that want native-rate decode e.g. seek-one-frame).
    #[test]
    fn non_positive_rate_omits_r_entirely() {
        let args = decode_args(Path::new("v.mp4"), 0.0, false, None, None, None, "nv12");
        assert!(!args.iter().any(|a| a == "-r"), "unexpected -r in {args:?}");
    }
}
