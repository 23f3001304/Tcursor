//! Pause-aware VFR recording sink for the legacy (compatibility) capture path.
//!
//! That path stamps the video by handing raw BGRA to ffmpeg's rawvideo demuxer under
//! `-use_wallclock_as_timestamps 1`, which timestamps each frame with the instant **ffmpeg**
//! reads it. There is therefore no timestamp to rebase the way the GPU path rebases
//! `send_frame`: a paused span - during which nothing is written - is baked into `video.mp4` as
//! a real PTS gap, and no ffmpeg option can take it back out, because a write can only ever
//! happen at or after the moment the frame was captured (compressing the timeline in-stream
//! would mean writing into the past). Meanwhile `sync.json`, the WAVs and every input stream DO
//! drop the pause, so the export - which reads `video.mp4` 1:1 against `sync.json`'s clock -
//! gets a pause-length frozen span, everything else running ahead of the picture from the
//! resume on, and a truncated tail (finding C1).
//!
//! So each unpaused span becomes its own MP4 part and `finish` concat-copies the parts into one
//! file: the paused spans never exist in it at all. Each part carries an explicit `duration`
//! directive taken from the recording clock, so the parts land exactly where `sync.json` says
//! (see `concat_list`). **Part 0 IS `out_path`**, so a recording that is never paused writes
//! exactly one file and finishes with no remux and no extra cost - the un-paused behaviour is
//! byte-for-byte what it was.
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use crate::capture::frame::Frame;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::win::sys::proc::ffcmd;

/// One recorded span: its file, and the recording-clock timestamp of its first encoded frame
/// (`None` until one arrives).
pub struct Part {
    pub path: PathBuf,
    pub first_ms: Option<u64>,
}

/// Path of segment `n` beside `out` (`video.mp4` -> `video.part1.mp4`). Segment 0 is normally
/// `out` itself, so `n` is normally >= 1 - but a leading span that never received a frame is
/// dropped, after which the numbering can restart at 0.
pub fn part_path(out: &Path, n: usize) -> PathBuf {
    let stem = out.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = out.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "mp4".into());
    out.with_file_name(format!("{stem}.part{n}.{ext}"))
}

/// A path as one concat-demuxer single-quoted token. Backslashes become forward slashes first,
/// so a Windows separator is never read as an escape; a literal `'` is then written as `'\''` -
/// close the quote, escape the quote, reopen it. Inside a single-quoted token ffmpeg's
/// `av_get_token` treats a backslash as an ORDINARY character, so the obvious `\'` ends the
/// token early and silently truncates the path (verified against the bundled ffmpeg: a path
/// under `O'Brien` resolved to `O\Brien` and the join failed).
fn quote(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").replace('\'', "'\\''")
}

/// The ffmpeg concat-demuxer list for `parts`, one `file '<path>'` line each, in order.
///
/// Every part but the last also carries a `duration` directive, which is what the demuxer
/// offsets the FOLLOWING file by. Without it the offset comes from the part's own container
/// duration, whose final-frame length is a VFR guess (measured at ~50 ms with this encoder) -
/// so each pause boundary drifted by that much and the error accumulated across pauses against
/// `full_dur_ms`. The directive is instead the recording clock's own span between consecutive
/// parts' first frames, which makes the join land exactly where `sync.json` says it should.
///
/// The span is always at least the part's own content extent plus the boundary gap, so it can
/// never place the next part on top of this one.
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
        Ok(Self { sink: Some(sink), out: out.clone(), parts: vec![Part { path: out, first_ms: None }], width, height })
    }

    fn close_current(&mut self) -> io::Result<()> {
        let Some(s) = self.sink.take() else { return Ok(()) };
        let finished = Box::new(s).finish();
        // ffmpeg exits non-zero when its input carried no frames at all. That is not a
        // recording failure - it is a span the user paused straight back out of - so the part
        // is dropped rather than failing the take, and a zero-frame MP4 never reaches the
        // concat list, where it has no first-frame timestamp to build a `duration` from.
        if self.parts.last().is_some_and(|p| p.first_ms.is_none()) {
            if let Some(p) = self.parts.pop() { let _ = std::fs::remove_file(&p.path); }
            return Ok(());
        }
        finished
    }

    /// `-c copy` remux of the parts over `out`. Written to a sibling first and renamed only on
    /// success, so a failed join leaves `video.mp4` as the valid - if short - first part plus
    /// every other part still on disk, rather than nothing at all.
    fn join_parts(&self) -> io::Result<()> {
        let list = self.out.with_extension("parts.txt");
        std::fs::write(&list, concat_list(&self.parts))?;
        let joined = self.out.with_extension("joined.mp4");
        let status = ffcmd("ffmpeg")
            .args(["-y", "-hide_banner", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i"])
            .arg(&list)
            .args(["-c", "copy"])
            .arg(&joined)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        let _ = std::fs::remove_file(&list);
        if !status.success() {
            let _ = std::fs::remove_file(&joined); // a half-written join is not salvage; the parts are
            return Err(io::Error::new(io::ErrorKind::Other, format!(
                "joining {} recording segments failed ({status}); video.mp4 holds only the span before the first pause",
                self.parts.len())));
        }
        std::fs::rename(&joined, &self.out)?;
        for p in self.parts.iter().skip(1) { let _ = std::fs::remove_file(&p.path); }
        Ok(())
    }
}

impl FrameSink for VfrSegments {
    /// `f.ts` is the RECORDING clock (`RecordingSession::pump_once` rebases it before pushing),
    /// which is what makes the first-frame timestamps recorded here the right basis for the
    /// concat offsets.
    fn push(&mut self, f: &Frame) -> io::Result<bool> {
        let Some(s) = self.sink.as_mut() else { return Ok(false) };
        let written = s.push(f)?;
        if written {
            if let Some(p) = self.parts.last_mut() { p.first_ms.get_or_insert(f.ts.0); }
        }
        Ok(written)
    }

    fn split(&mut self) -> io::Result<()> {
        self.close_current()?;
        let next = part_path(&self.out, self.parts.len());
        self.sink = Some(FfmpegFrameSink::new_vfr(&next.to_string_lossy(), self.width, self.height)?);
        self.parts.push(Part { path: next, first_ms: None });
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.close_current()?;
        match self.parts.len() {
            0 => Ok(()), // not one frame was ever encoded; there is nothing to write
            // One span, so it IS the whole recording - normally because the take was never
            // paused, and it is a later part only if every span before it was empty.
            1 if self.parts[0].path != self.out => std::fs::rename(&self.parts[0].path, &self.out),
            1 => Ok(()),
            _ => self.join_parts(),
        }
    }
}

#[cfg(test)]
#[path = "vfr_segments_tests.rs"]
mod tests;
