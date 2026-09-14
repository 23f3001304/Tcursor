//! Merge a take's webcam segments back into the single `webcam.webm` the editor's stage and the
//! export's `WebcamPipe` already read (mid-take source switching, 2026-09-14). A camera switch
//! writes `webcam_2.webm`, `webcam_3.webm`... beside the first file; this runs once, from
//! `preprocess::essential`, before the editor opens, so nothing downstream ever learns a second
//! camera existed. A take with no switch has an empty `sync.webcam_segments` and pays nothing.
use std::path::{Path, PathBuf};

use crate::export::pipeline::ffio::{probe_dims, probe_duration};
use crate::session::paths::ProjectPaths;
use crate::session::sync::SyncLog;
use crate::win::sys::proc::{ffcmd_bg, tmp_sibling};

/// Output frame rate of the merged file. The black gap sources need a rate, and `concat` needs
/// every input at one rate, so the merge picks the webcam's own ceiling rather than a segment's
/// measured (and variable) one.
const MERGE_FPS: u32 = 30;

/// One segment as the merge sees it: the file, where it starts on the recording clock, how long it
/// actually ran, and its pixel size. The FIRST part's `start_ms` is 0 by the same convention the
/// export already aligns `webcam.webm` with (its first frame is the take's start); every later
/// part's is the switch instant `mark_webcam_segment` stamped.
#[derive(Clone, Debug, PartialEq)]
pub struct Part { pub file: String, pub start_ms: u64, pub dur_ms: u64, pub w: u32, pub h: u32 }

/// The gap (ms) in front of each part: the gap of part 0 is always 0, and part k's is how long the
/// camera was dark between the previous part's end and this part's start (device teardown and
/// setup, typically 100 to 300 ms). Saturating, so a stamp that lands inside the previous segment
/// (a chunk still flushing at the switch) closes the gap rather than going negative.
pub fn gaps(parts: &[Part]) -> Vec<u64> {
    let mut out = Vec::with_capacity(parts.len());
    let mut end = 0u64;
    for (i, p) in parts.iter().enumerate() {
        out.push(if i == 0 { 0 } else { p.start_ms.saturating_sub(end) });
        end = p.start_ms.max(end) + p.dur_ms;
    }
    out
}

/// The whole ffmpeg argument list for the merge, or `None` when there is nothing to merge (fewer
/// than two parts). Pure, so the filter graph is unit-tested without shelling out. Every part is
/// scaled and padded into the FIRST part's size (a second camera is routinely another resolution),
/// normalised to one rate, SAR and pixel format, and a `color` source fills each gap with black so
/// the merged file's timeline still matches the take's.
pub fn merge_args(parts: &[Part], out: &Path) -> Option<Vec<String>> {
    if parts.len() < 2 { return None; }
    let (w, h) = (parts[0].w.max(2), parts[0].h.max(2));
    let gaps = gaps(parts);
    let mut graph = String::new();
    let mut order = String::new();
    let mut n = parts.len();
    for i in 0..parts.len() {
        if gaps[i] > 0 {
            n += 1;
            graph.push_str(&format!(
                "color=c=black:size={w}x{h}:rate={MERGE_FPS}:duration={:.3},format=yuv420p,setsar=1[g{i}];", ms(gaps[i])));
            order.push_str(&format!("[g{i}]"));
        }
        graph.push_str(&format!(
            "[{i}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={MERGE_FPS},format=yuv420p[v{i}];"));
        order.push_str(&format!("[v{i}]"));
    }
    graph.push_str(&format!("{order}concat=n={n}:v=1:a=0[out]"));
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
    for p in parts { args.push("-i".into()); args.push(p.file.clone()); }
    args.extend(["-filter_complex".into(), graph, "-map".into(), "[out]".into(), "-an".into(),
        "-c:v".into(), "libvpx-vp9".into(), "-b:v".into(), "0".into(), "-crf".into(), "32".into(),
        "-row-mt".into(), "1".into(), "-deadline".into(), "good".into(), "-cpu-used".into(), "4".into(),
        out.to_string_lossy().into_owned()]);
    Some(args)
}

fn ms(v: u64) -> f64 { v as f64 / 1000.0 }

/// Describe the files on disk: the first segment (`webcam.webm`, anchored at 0) then every extra
/// segment `sync` lists, skipping any whose file is gone or unprobeable rather than failing the
/// whole merge over one lost camera.
fn parts_on_disk(paths: &ProjectPaths, sync: &SyncLog) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut push = |file: PathBuf, start_ms: u64| {
        if !file.exists() { return; }
        let (Ok((w, h)), Ok(secs)) = (probe_dims(&file), probe_duration(&file)) else { return };
        parts.push(Part { file: file.to_string_lossy().into_owned(), start_ms, dur_ms: (secs * 1000.0) as u64, w, h });
    };
    push(paths.webcam(), 0);
    for s in &sync.webcam_segments { push(paths.folder.join(&s.path), s.start_ms); }
    parts
}

/// Replace `webcam.webm` with every segment merged in order. A no-op for a take without a camera
/// switch. The originals move to `<folder>/segments/` first, so the merge is re-runnable by hand
/// and a bad merge never destroys a take's camera track; the merged file is written to a temp
/// sibling and renamed into place, so no reader ever sees a half-written `webcam.webm`.
pub fn merge_webcam_segments(paths: &ProjectPaths, sync: &SyncLog) -> Result<(), String> {
    if sync.webcam_segments.is_empty() { return Ok(()); }
    let parts = parts_on_disk(paths, sync);
    let out = paths.webcam();
    let tmp = tmp_sibling(&out);
    let Some(args) = merge_args(&parts, &tmp) else { return Ok(()) };
    let status = ffcmd_bg("ffmpeg").args(&args).status().map_err(|e| e.to_string())?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("webcam merge failed ({} segments)", parts.len()));
    }
    let keep = paths.folder.join("segments");
    std::fs::create_dir_all(&keep).map_err(|e| e.to_string())?;
    for p in &parts {
        let src = PathBuf::from(&p.file);
        if let Some(name) = src.file_name() { let _ = std::fs::rename(&src, keep.join(name)); }
    }
    std::fs::rename(&tmp, &out).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "segments_webcam_tests.rs"]
mod tests;
