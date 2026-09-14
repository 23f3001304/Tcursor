// Fold a take's extra microphone segments back into the one `mic.wav` the editor and the export
// already read (2026-09-14 mid-take source switching). Every `switch_mic` ends one WAV and opens
// the next, so a take with two switches has `mic.wav`, `mic_2.wav` and `mic_3.wav` plus the
// recording-clock instant each of them started at; this runs once, from `preprocess::essential`,
// before the editor opens, and leaves exactly the single continuous WAV that was there before
// switching existed. Silence between segments (the ~50ms of device teardown/setup, or a stretch
// with the mic switched off entirely) is what the delays produce, so the merged track keeps every
// later word where it was spoken. The originals are moved to `segments/`, never deleted, so a
// re-merge is always possible and a bad merge can never cost the take its narration.
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::session::paths::ProjectPaths;
use crate::session::sync::SyncLog;
use crate::win::sys::proc::{ffcmd_bg, tmp_sibling};

/// One WAV going into the merge: where it is, and how far into the merged track it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct MicPart { pub path: PathBuf, pub delay_ms: u64 }

/// ffmpeg's `channel_layouts=` name for a channel count. Every real input device is mono or
/// stereo; the `<n>c` form covers the rest rather than guessing a speaker arrangement.
fn layout(channels: u16) -> String {
    match channels { 1 => "mono".into(), 2 => "stereo".into(), n => format!("{n}c") }
}

/// The ffmpeg args (appended after the shared `-y -v error`) that merge `parts` into `out`, pure
/// so the exact command is pinned by tests instead of discovered by ear on a real recording.
/// Every part is forced to `rate`/`channels` - the FIRST part's own spec - because `amix` refuses
/// inputs that disagree, and a mid-take switch routinely lands on a device with another rate.
pub fn merge_args(parts: &[MicPart], rate: u32, channels: u16, out: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    for p in parts { args.push("-i".into()); args.push(p.path.as_os_str().to_os_string()); }
    let (lay, n) = (layout(channels), parts.len());
    let branches: Vec<String> = parts.iter().enumerate().map(|(i, p)| {
        // `adelay` takes one delay per channel; a zero delay emits no filter at all so the first
        // part's branch stays the plain conversion it is.
        let delay = if p.delay_ms == 0 { String::new() } else {
            let each: Vec<String> = (0..channels.max(1)).map(|_| p.delay_ms.to_string()).collect();
            format!(",adelay={}", each.join("|"))
        };
        let label = if n == 1 { "[a]".to_string() } else { format!("[s{i}]") };
        format!("[{i}:a]aresample={rate},aformat=channel_layouts={lay}{delay}{label}")
    }).collect();
    let mut filter = branches.join(";");
    if n > 1 {
        // `normalize=0` keeps each segment at its recorded level (the default would scale every
        // sample by 1/n); `dropout_transition=0` stops amix fading around a segment's end.
        let ins: String = (0..n).map(|i| format!("[s{i}]")).collect();
        filter.push_str(&format!(";{ins}amix=inputs={n}:normalize=0:dropout_transition=0[a]"));
    }
    args.extend(["-filter_complex", &filter, "-map", "[a]", "-c:a", "pcm_s16le"].map(OsString::from));
    args.push(out.as_os_str().to_os_string());
    args
}

/// The merge's inputs in recording order, skipping every segment with no file on disk - a switch
/// TO no mic logs its boundary but writes nothing, and a device that would not open has had its
/// header-only WAV removed by `spawn_mic_thread`.
///
/// The origin every delay is measured from is `mic_ms` (where `sync.json` places `mic.wav`), or -
/// when the take started with the mic off, so there is no first segment and no `mic_ms` - the
/// video's own first frame, which is exactly where `audio_shift_ms` places a track with no start.
fn parts_of(paths: &ProjectPaths, sync: &SyncLog) -> Vec<MicPart> {
    let origin = sync.mic_ms.or_else(|| sync.frames.first().copied()).unwrap_or(0);
    let mut parts = Vec::new();
    if paths.mic().exists() { parts.push(MicPart { path: paths.mic(), delay_ms: 0 }); }
    for s in &sync.mic_segments {
        let path = paths.folder.join(&s.path);
        if path.exists() { parts.push(MicPart { path, delay_ms: s.start_ms.saturating_sub(origin) }); }
    }
    parts
}

/// A WAV's sample rate and channel count, read from its own header rather than probed through a
/// subprocess - the merge needs the first segment's spec, and `hound` already writes these files.
fn wav_spec(path: &Path) -> Result<(u32, u16), String> {
    let spec = hound::WavReader::open(path).map_err(|e| format!("read {path:?}: {e}"))?.spec();
    Ok((spec.sample_rate, spec.channels))
}

/// `ffmpeg -y -v error <args>` at below-normal priority, like every other preprocess pass.
fn run(args: &[OsString]) -> Result<(), String> {
    let status = ffcmd_bg("ffmpeg").args(["-y", "-v", "error"]).args(args)
        .stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map_err(|e| format!("spawn ffmpeg (mic merge): {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("ffmpeg mic merge exited with {status}")) }
}

/// Merge every extra mic segment `sync` lists into `mic.wav`. A take with no mid-take mic switch
/// has an empty list and this does nothing at all. Safe to run twice: the second pass finds the
/// originals already moved to `segments/` and returns before touching anything.
pub fn merge_mic_segments(paths: &ProjectPaths, sync: &SyncLog) -> Result<(), String> {
    if sync.mic_segments.is_empty() { return Ok(()); }
    let parts = parts_of(paths, sync);
    let first = match parts.first() { Some(p) => p, None => return Ok(()) };
    if parts.len() == 1 && first.delay_ms == 0 && first.path == paths.mic() { return Ok(()); }
    let (rate, channels) = wav_spec(&first.path)?;
    let (out, tmp) = (paths.mic(), tmp_sibling(&paths.mic()));
    if let Err(e) = run(&merge_args(&parts, rate, channels, &tmp)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    // Originals aside FIRST: the merged file takes `mic.wav`'s name, so the first segment has to
    // be out of the way before the rename, and it must land somewhere recoverable rather than
    // under the new file. A failed rename puts it straight back.
    let kept = paths.folder.join("segments");
    std::fs::create_dir_all(&kept).map_err(|e| format!("create {kept:?}: {e}"))?;
    for p in parts.iter().map(|p| &p.path) {
        if let Some(n) = p.file_name() { let _ = std::fs::rename(p, kept.join(n)); }
    }
    std::fs::rename(&tmp, &out).map_err(|e| {
        let _ = std::fs::rename(kept.join("mic.wav"), &out);
        format!("rename {tmp:?} -> {out:?}: {e}")
    })
}

#[cfg(test)]
#[path = "segments_audio_tests.rs"]
mod tests;
