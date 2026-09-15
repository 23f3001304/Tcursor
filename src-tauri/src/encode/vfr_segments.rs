use crate::capture::frame::Frame;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::process::proc::ffcmd;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub struct Part {
    pub path: PathBuf,
    pub first_ms: Option<u64>,
}

pub fn part_path(out: &Path, n: usize) -> PathBuf {
    let stem = out
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = out
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "mp4".into());
    out.with_file_name(format!("{stem}.part{n}.{ext}"))
}

fn quote(p: &Path) -> String {
    p.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''")
}

pub fn concat_list(parts: &[Part]) -> String {
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        out.push_str(&format!("file '{}'\n", quote(&p.path)));
        if let (Some(a), Some(b)) = (p.first_ms, parts.get(i + 1).and_then(|n| n.first_ms)) {
            let ms = b.saturating_sub(a);
            out.push_str(&format!("duration {}.{:03}\n", ms / 1000, ms % 1000));
        }
    }
    out
}

pub struct VfrSegments {
    sink: Option<FfmpegFrameSink>,
    out: PathBuf,
    parts: Vec<Part>,
    width: u32,
    height: u32,
}

impl VfrSegments {
    pub fn new(out_path: &str, width: u32, height: u32) -> io::Result<Self> {
        let out = PathBuf::from(out_path);
        let sink = FfmpegFrameSink::new_vfr(out_path, width, height)?;
        Ok(Self {
            sink: Some(sink),
            out: out.clone(),
            parts: vec![Part {
                path: out,
                first_ms: None,
            }],
            width,
            height,
        })
    }

    fn close_current(&mut self) -> io::Result<()> {
        let Some(s) = self.sink.take() else {
            return Ok(());
        };
        let finished = Box::new(s).finish();
        if self.parts.last().is_some_and(|p| p.first_ms.is_none()) {
            if let Some(p) = self.parts.pop() {
                let _ = std::fs::remove_file(&p.path);
            }
            return Ok(());
        }
        finished
    }

    fn join_parts(&self) -> io::Result<()> {
        let list = self.out.with_extension("parts.txt");
        std::fs::write(&list, concat_list(&self.parts))?;
        let joined = self.out.with_extension("joined.mp4");
        let status = ffcmd("ffmpeg")
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
            ])
            .arg(&list)
            .args(["-c", "copy"])
            .arg(&joined)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        let _ = std::fs::remove_file(&list);
        if !status.success() {
            let _ = std::fs::remove_file(&joined);
            return Err(io::Error::new(io::ErrorKind::Other, format!(
                "joining {} recording segments failed ({status}); video.mp4 holds only the span before the first pause",
                self.parts.len())));
        }
        std::fs::rename(&joined, &self.out)?;
        for p in self.parts.iter().skip(1) {
            let _ = std::fs::remove_file(&p.path);
        }
        Ok(())
    }
}

impl FrameSink for VfrSegments {
    fn push(&mut self, f: &Frame) -> io::Result<bool> {
        let Some(s) = self.sink.as_mut() else {
            return Ok(false);
        };
        let written = s.push(f)?;
        if written {
            if let Some(p) = self.parts.last_mut() {
                p.first_ms.get_or_insert(f.ts.0);
            }
        }
        Ok(written)
    }

    fn split(&mut self) -> io::Result<()> {
        self.close_current()?;
        let next = part_path(&self.out, self.parts.len());
        self.sink = Some(FfmpegFrameSink::new_vfr(
            &next.to_string_lossy(),
            self.width,
            self.height,
        )?);
        self.parts.push(Part {
            path: next,
            first_ms: None,
        });
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.close_current()?;
        match self.parts.len() {
            0 => Ok(()),
            1 if self.parts[0].path != self.out => std::fs::rename(&self.parts[0].path, &self.out),
            1 => Ok(()),
            _ => self.join_parts(),
        }
    }
}

#[cfg(test)]
#[path = "vfr_segments_tests.rs"]
mod tests;
