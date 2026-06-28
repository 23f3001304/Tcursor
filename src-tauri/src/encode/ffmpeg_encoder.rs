use crate::capture::frame::Frame;
use crate::encode::frame_sink::FrameSink;
use std::io::Write;
use std::process::{Child, Stdio};
use crate::win::proc::ffcmd;
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
    ffcmd("ffmpeg")
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

/// Force the encoder probe now (the first ffmpeg launch also lets the OS finish
/// scanning the bundled binary). Call on a background thread at startup so the
/// first recording's sink creation is fast and audio capture isn't delayed behind
/// it. Cached afterward, so later calls are free.
pub fn prewarm() {
    let enc = h264_encoder();
    // Record whether ffmpeg actually launches (the encoder probe silently swallows a
    // missing/AV-blocked binary) into the same diagnostic log written at startup.
    let runs = ffcmd("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true)
        .open(std::env::temp_dir().join("tcursor-ffmpeg.log"))
    {
        let _ = f.write_all(format!("encoder={enc} ffmpeg_runs={runs:?}\n").as_bytes());
    }
}

impl FfmpegFrameSink {
    /// Realtime-quality encode (used while recording).
    pub fn new(out_path: &str, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps as f64, false)
    }

    /// High-quality encode for offline export (slower preset, near-lossless).
    /// `fps` is f64 so the exporter can reinterpret a mislabeled source rate.
    pub fn new_hq(out_path: &str, width: u32, height: u32, fps: f64) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps, true)
    }

    fn spawn(out_path: &str, width: u32, height: u32, fps: f64, hq: bool) -> std::io::Result<Self> {
        let encoder = h264_encoder();
        let size = format!("{width}x{height}");
        let fr = format!("{fps:.4}");
        let mut cmd = ffcmd("ffmpeg");
        cmd.args([
            "-y", "-f", "rawvideo", "-pixel_format", "bgra",
            "-video_size", &size, "-framerate", &fr,
            "-i", "pipe:0", "-c:v", encoder, "-pix_fmt", "yuv420p",
        ]);
        match (encoder, hq) {
            ("libx264", true) => cmd.args(["-preset", "slow", "-crf", "16"]),
            ("libx264", false) => cmd.args(["-preset", "ultrafast", "-crf", "26"]),
            (_, true) => cmd.args(["-preset", "p5", "-rc", "vbr", "-cq", "20", "-b:v", "0"]),
            (_, false) => cmd.args(["-b:v", "16M"]),
        };
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
