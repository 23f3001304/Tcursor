use crate::capture::frame::Frame;
use crate::encode::frame_sink::FrameSink;
use crate::export::settings::Format;
use crate::process::proc::ffcmd;
use std::io::Write;
use std::process::{Child, Stdio};
use std::sync::OnceLock;

pub struct FfmpegFrameSink {
    child: Child,
    width: u32,
    height: u32,
    warned: bool,
}

fn write_or_skip(
    w: &mut dyn Write,
    f: &Frame,
    expected: (u32, u32),
    warned: &mut bool,
) -> std::io::Result<bool> {
    if (f.width, f.height) != expected {
        if !*warned {
            eprintln!(
                "ffmpeg sink: frame {}x{} != expected {}x{}, skipping (window resized mid-record?)",
                f.width, f.height, expected.0, expected.1
            );
            *warned = true;
        }
        return Ok(false);
    }
    w.write_all(&f.bgra)?;
    Ok(true)
}

fn probe_encoder(encoder: &str) -> bool {
    ffcmd("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x240:d=1",
            "-frames:v",
            "1",
            "-c:v",
            encoder,
            "-pix_fmt",
            "yuv420p",
            "-f",
            "null",
            "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn h264_encoder() -> &'static str {
    static ENCODER: OnceLock<String> = OnceLock::new();
    ENCODER.get_or_init(|| {
        for encoder in ["h264_nvenc", "h264_qsv", "h264_amf", "h264_mf"] {
            if probe_encoder(encoder) {
                return encoder.to_string();
            }
        }
        "libx264".to_string()
    })
}

pub fn prewarm() {
    let encoder = h264_encoder();
    let runs = ffcmd("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let log = std::env::temp_dir().join("tcursor-ffmpeg.log");
    let full = std::fs::metadata(&log)
        .map(|m| m.len() > 64 * 1024)
        .unwrap_or(false);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(!full)
        .write(full)
        .truncate(full)
        .open(&log)
    {
        let _ = f.write_all(format!("encoder={encoder} ffmpeg_runs={runs:?}\n").as_bytes());
    }
}

impl FfmpegFrameSink {
    pub fn new(out_path: &str, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps as f64, false, false)
    }

    pub fn new_vfr(out_path: &str, width: u32, height: u32) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, 0.0, false, true)
    }

    pub fn new_medium(
        out_path: &str,
        width: u32,
        height: u32,
        fps: f64,
        format: Format,
        crf: u8,
    ) -> std::io::Result<Self> {
        let encoder = if matches!(format, Format::Mp4) {
            h264_encoder()
        } else {
            ""
        };
        let args = crate::encode::ffmpeg_args::export_args(
            format, encoder, width, height, fps, crf, out_path,
        );
        let child = ffcmd("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            width,
            height,
            warned: false,
        })
    }

    pub fn new_hq(out_path: &str, width: u32, height: u32, fps: f64) -> std::io::Result<Self> {
        Self::spawn(out_path, width, height, fps, true, false)
    }

    fn spawn(
        out_path: &str,
        width: u32,
        height: u32,
        fps: f64,
        hq: bool,
        vfr: bool,
    ) -> std::io::Result<Self> {
        let encoder = h264_encoder();
        let size = format!("{width}x{height}");
        let fr = format!("{fps:.4}");
        let mut cmd = ffcmd("ffmpeg");
        cmd.args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "bgra",
            "-video_size",
            &size,
        ]);
        if vfr {
            cmd.args(["-use_wallclock_as_timestamps", "1"]);
        } else {
            cmd.args(["-framerate", &fr]);
        }
        cmd.args(["-i", "pipe:0", "-c:v", encoder, "-pix_fmt", "yuv420p"]);
        if vfr {
            cmd.args(["-fps_mode", "passthrough"]);
        }
        match (encoder, hq) {
            ("libx264", true) => cmd.args(["-preset", "medium", "-crf", "18"]),
            ("libx264", false) => cmd.args(["-preset", "veryfast", "-crf", "24"]),
            ("h264_nvenc", true) => cmd.args([
                "-preset", "p3", "-rc", "vbr", "-cq", "18", "-b:v", "0", "-bf", "0",
            ]),
            ("h264_nvenc", false) => cmd.args([
                "-preset", "p2", "-rc", "vbr", "-cq", "24", "-b:v", "12M", "-bf", "0",
            ]),
            ("h264_qsv", true) => cmd.args(["-preset", "faster", "-global_quality", "20"]),
            ("h264_qsv", false) => cmd.args(["-preset", "veryfast", "-b:v", "12M"]),
            ("h264_amf", true) => cmd.args([
                "-quality", "speed", "-rc", "cqp", "-qp_p", "20", "-qp_i", "20",
            ]),
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
        Ok(Self {
            child,
            width,
            height,
            warned: false,
        })
    }
}

impl FrameSink for FfmpegFrameSink {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool> {
        let stdin = self.child.stdin.as_mut().expect("ffmpeg stdin");
        write_or_skip(stdin, f, (self.width, self.height), &mut self.warned)
    }
    fn finish(mut self: Box<Self>) -> std::io::Result<()> {
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
