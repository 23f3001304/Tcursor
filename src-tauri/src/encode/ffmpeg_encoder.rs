use crate::capture::frame::Frame;
use crate::encode::frame_sink::FrameSink;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

/// Pipes BGRA frames to a system ffmpeg process that writes an H.264 MP4,
/// using a GPU hardware encoder when available (falls back to libx264).
pub struct FfmpegFrameSink {
    child: Child,
    width: u32,
    height: u32,
}

/// Encode one test frame to verify the encoder actually works on this machine.
fn probe_encoder(enc: &str) -> bool {
    Command::new("ffmpeg")
        .args([
            "-hide_banner", "-loglevel", "error",
            "-f", "lavfi", "-i", "color=c=black:s=320x240:d=1",
            "-frames:v", "1", "-c:v", enc, "-pix_fmt", "yuv420p",
            "-f", "null", "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Pick the best available H.264 encoder once, preferring hardware.
fn h264_encoder() -> &'static str {
    static ENCODER: OnceLock<String> = OnceLock::new();
    ENCODER.get_or_init(|| {
        for enc in ["h264_nvenc", "h264_qsv", "h264_amf", "h264_mf"] {
            if probe_encoder(enc) {
                return enc.to_string();
            }
        }
        "libx264".to_string()
    })
}

impl FfmpegFrameSink {
    pub fn new(out_path: &str, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        let encoder = h264_encoder();
        let size = format!("{width}x{height}");
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-y",
            "-f", "rawvideo",
            "-pixel_format", "bgra",
            "-video_size", &size,
            "-framerate", &fps.to_string(),
            "-i", "pipe:0",
            "-c:v", encoder,
            "-pix_fmt", "yuv420p",
        ]);
        if encoder == "libx264" {
            cmd.args(["-preset", "ultrafast", "-crf", "26"]);
        } else {
            cmd.args(["-b:v", "16M"]);
        }
        let child = cmd
            .arg(out_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child, width, height })
    }
}

impl FrameSink for FfmpegFrameSink {
    fn push(&mut self, f: &Frame) -> std::io::Result<()> {
        debug_assert_eq!((f.width, f.height), (self.width, self.height));
        let stdin = self.child.stdin.as_mut().expect("ffmpeg stdin");
        stdin.write_all(&f.bgra)
    }
    fn finish(mut self: Box<Self>) -> std::io::Result<()> {
        // Closing stdin signals EOF so ffmpeg flushes and exits.
        drop(self.child.stdin.take());
        let status = self.child.wait()?;
        if !status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("ffmpeg exited with {status}"),
            ));
        }
        Ok(())
    }
}
