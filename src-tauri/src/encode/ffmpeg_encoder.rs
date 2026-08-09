use crate::capture::frame::Frame;
use crate::encode::frame_sink::FrameSink;
use std::io::Write;
use std::process::{Child, Stdio};
use crate::win::sys::proc::ffcmd;
use crate::export::settings::Format;
use std::sync::OnceLock;

/// Pipes BGRA frames to a system ffmpeg process that writes an H.264 MP4,
/// using a GPU hardware encoder when available (falls back to libx264).
pub struct FfmpegFrameSink {
    child: Child,
    width: u32,
    height: u32,
    warned: bool, // latches true after the first mid-record dimension-mismatch warning
}

/// A mid-record resize corrupts the rawvideo pipe's fixed frame size; skip a mismatched
/// frame (never write partial/misaligned bytes into it) and warn once via `warned`
/// instead of spamming - a frozen-then-recovered video beats a corrupt stream. `w` is
/// generic (not tied to `ChildStdin`) so this is unit-testable with a plain `Vec<u8>`.
fn write_or_skip(w: &mut dyn Write, f: &Frame, expected: (u32, u32), warned: &mut bool) -> std::io::Result<bool> {
    if (f.width, f.height) != expected {
        if !*warned {
            eprintln!("ffmpeg sink: frame {}x{} != expected {}x{}, skipping (window resized mid-record?)", f.width, f.height, expected.0, expected.1);
            *warned = true;
        }
        return Ok(false);
    }
    w.write_all(&f.bgra)?;
    Ok(true)
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
    /// Realtime CFR encode (game mode): a steady `fps` so variable-FPS sources record smoothly.
    pub fn new(out_path: &str, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps as f64, false, false)
    }

    /// Realtime VFR encode (normal recording): each frame is stamped with its real arrival
    /// (wall-clock) time, so `video.mp4` plays at true speed even when the capture rate dips below
    /// the display refresh (a fixed-`-framerate` CFR file would otherwise play sped up).
    pub fn new_vfr(out_path: &str, width: u32, height: u32) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, 0.0, false, true)
    }

    /// Encode for offline export (`exporter::export`'s only caller): `format` picks the
    /// container + codec (MP4/H.264 - today's only path, hardware-first via `h264_encoder()` -
    /// WebM/VP9, or GIF via a palettegen/paletteuse filter chain) and `crf` (18..28,
    /// `settings::DEFAULT_CRF` = 24) drives quality where a knob exists. Arg construction is
    /// pure (`ffmpeg_args::export_args`, unit-tested without spawning ffmpeg); this function only
    /// resolves the H.264 hardware encoder (when relevant) and spawns the process.
    pub fn new_medium(out_path: &str, width: u32, height: u32, fps: f64, format: Format, crf: u8) -> std::io::Result<Self> {
        let encoder = if matches!(format, Format::Mp4) { h264_encoder() } else { "" };
        let args = crate::encode::ffmpeg_args::export_args(format, encoder, width, height, fps, crf, out_path);
        let child = ffcmd("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child, width, height, warned: false })
    }

    /// High-quality CFR encode for offline export (slower preset, near-lossless).
    /// `fps` is f64 so the exporter can reinterpret a mislabeled source rate.
    pub fn new_hq(out_path: &str, width: u32, height: u32, fps: f64) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps, true, false)
    }

    fn spawn(out_path: &str, width: u32, height: u32, fps: f64, hq: bool, vfr: bool) -> std::io::Result<Self> {
        let encoder = h264_encoder();
        let size = format!("{width}x{height}");
        let fr = format!("{fps:.4}");
        let mut cmd = ffcmd("ffmpeg");
        cmd.args(["-y", "-f", "rawvideo", "-pixel_format", "bgra", "-video_size", &size]);
        // VFR: stamp each frame with its real arrival (wall-clock) time so the file plays at true
        // speed even when capture dips below the display refresh. CFR: a fixed input rate.
        // `passthrough` keeps every frame (no drops), so the frame<->sync.json map stays 1:1.
        if vfr { cmd.args(["-use_wallclock_as_timestamps", "1"]); } else { cmd.args(["-framerate", &fr]); }
        cmd.args(["-i", "pipe:0", "-c:v", encoder, "-pix_fmt", "yuv420p"]);
        if vfr { cmd.args(["-fps_mode", "passthrough"]); }
        match (encoder, hq) {
            ("libx264", true) => cmd.args(["-preset", "medium", "-crf", "18"]),
            ("libx264", false) => cmd.args(["-preset", "veryfast", "-crf", "24"]),
            ("h264_nvenc", true) => cmd.args(["-preset", "p3", "-rc", "vbr", "-cq", "18", "-b:v", "0", "-bf", "0"]),
            ("h264_nvenc", false) => cmd.args(["-preset", "p2", "-rc", "vbr", "-cq", "24", "-b:v", "12M", "-bf", "0"]),
            ("h264_qsv", true) => cmd.args(["-preset", "faster", "-global_quality", "20"]),
            ("h264_qsv", false) => cmd.args(["-preset", "veryfast", "-b:v", "12M"]),
            ("h264_amf", true) => cmd.args(["-quality", "speed", "-rc", "cqp", "-qp_p", "20", "-qp_i", "20"]),
            ("h264_amf", false) => cmd.args(["-quality", "speed", "-b:v", "12M"]),
            ("h264_mf", true) => cmd.args(["-b:v", "20M"]),
            ("h264_mf", false) => cmd.args(["-b:v", "12M"]),
            (_, true) => cmd.args(["-crf", "20"]),
            (_, false) => cmd.args(["-b:v", "12M"]),
        };
        let child = cmd
            .arg(out_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self { child, width, height, warned: false })
    }
}

impl FrameSink for FfmpegFrameSink {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool> {
        let stdin = self.child.stdin.as_mut().expect("ffmpeg stdin");
        write_or_skip(stdin, f, (self.width, self.height), &mut self.warned)
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

#[cfg(test)]
#[path = "ffmpeg_encoder_tests.rs"]
mod tests;
