// Remove silences: ffmpeg's `silencedetect` over the recorded tracks, intersected when both exist
// (a stretch is silent only when mic AND system are), mapped onto the clip clock with the same
// shift the mux uses, padded so speech never starts mid-word, filtered, clamped into the trim.
// The panel applies the result as ONE `AddCuts`, so Remove silences is one undo step. No model,
// no network: the thresholds are constants until someone asks for a settings row.
use anyhow::{Context, Result};
use std::path::Path;
use crate::export::pipeline::audio_shift_ms;
use crate::export::render::RenderMeta;
use crate::session::paths::ProjectPaths;
use crate::win::sys::proc::ffcmd;

/// Anything quieter than this, for at least `MIN_MS`, is a silence.
pub const THRESHOLD_DB: f32 = -35.0;
/// The shortest stretch worth cutting, measured AFTER padding.
pub const MIN_MS: u32 = 700;
/// Kept on each side of a silence so a word's tail and the next one's attack survive.
pub const PAD_MS: u32 = 150;

fn field(line: &str, key: &str) -> Option<f64> {
    let rest = &line[line.find(key)? + key.len()..];
    rest.split(|c: char| c == ' ' || c == '|').next()?.trim().parse().ok()
}

/// `silence_start: x` / `silence_end: y` pairs out of ffmpeg's log, in seconds of the track's own
/// clock. A start with no end (the file ends silent) runs to `f64::MAX`.
pub fn parse_silencedetect(stderr: &str) -> Vec<(f64, f64)> {
    let (mut out, mut open) = (Vec::new(), None::<f64>);
    for line in stderr.lines() {
        if let Some(v) = field(line, "silence_start: ") { open = Some(v); }
        else if let Some(v) = field(line, "silence_end: ") { if let Some(s) = open.take() { out.push((s, v)); } }
    }
    if let Some(s) = open { out.push((s, f64::MAX)); }
    out
}

/// Track seconds to clip ms: `track_shift_ms` is `audio_shift_ms(track_ms, video_start, 0)`, the
/// track's own start relative to the clip's first frame. An open end stays open.
pub fn to_clip_ms(spans_s: &[(f64, f64)], track_shift_ms: i64) -> Vec<(u32, u32)> {
    let f = |x: f64| if x == f64::MAX { u32::MAX } else { ((x * 1000.0).round() as i64 + track_shift_ms).max(0) as u32 };
    spans_s.iter().map(|&(a, b)| (f(a), f(b))).collect()
}

/// The stretches silent on BOTH tracks, sorted.
pub fn intersect(a: &[(u32, u32)], b: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for &(a0, a1) in a { for &(b0, b1) in b { let (s, e) = (a0.max(b0), a1.min(b1)); if s < e { out.push((s, e)); } } }
    out.sort_unstable();
    out
}

/// Clamp into `[lo, hi]` first, then pad each side, then drop what is shorter than `min_ms`.
pub fn pad_and_filter(spans: Vec<(u32, u32)>, pad_ms: u32, min_ms: u32, lo: u32, hi: u32) -> Vec<(u32, u32)> {
    spans.into_iter().filter_map(|(s, e)| {
        let (s, e) = (s.clamp(lo, hi).saturating_add(pad_ms), e.clamp(lo, hi).saturating_sub(pad_ms));
        (e > s && e - s >= min_ms).then_some((s, e))
    }).collect()
}

fn track_silences(track: &Path, shift_ms: i64) -> Result<Vec<(u32, u32)>> {
    let out = ffcmd("ffmpeg").args(["-v", "info", "-i"]).arg(track)
        .args(["-af", &format!("silencedetect=n={THRESHOLD_DB}dB:d={}", MIN_MS as f64 / 1000.0), "-f", "null", "-"])
        .output().with_context(|| format!("spawn ffmpeg silencedetect on {track:?}"))?;
    Ok(to_clip_ms(&parse_silencedetect(&String::from_utf8_lossy(&out.stderr)), shift_ms))
}

/// The silences of this recording as clip-time spans ready for `AddCuts`: whichever of mic and
/// system exist are scanned; with both, a stretch counts only when both are silent.
pub fn detect(paths: &ProjectPaths, meta: &RenderMeta) -> Result<Vec<(u32, u32)>> {
    let shift = |t: Option<u64>| audio_shift_ms(t, meta.video_start, 0);
    let mic = paths.mic().exists().then(|| track_silences(&paths.mic(), shift(meta.tl.mic_ms))).transpose()?;
    let sys = paths.system().exists().then(|| track_silences(&paths.system(), shift(meta.tl.system_ms))).transpose()?;
    let spans = match (mic, sys) { (Some(m), Some(s)) => intersect(&m, &s), (Some(v), None) | (None, Some(v)) => v, (None, None) => vec![] };
    let full = (meta.video_end - meta.video_start) as u32;
    let (lo, hi) = meta.trim.resolve(full);
    Ok(pad_and_filter(spans, PAD_MS, MIN_MS, lo, hi))
}

/// The editor's Remove silences: runs `detect` on the warm preview session's own meta.
#[tauri::command]
pub async fn detect_silences(folder: String, app: tauri::AppHandle) -> Result<Vec<(u32, u32)>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        crate::export::preview::with_warm(&app.state::<crate::export::preview::PreviewSession>(), &folder,
            |c, paths| detect(paths, &c.meta).map_err(|e| e.to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "silence_tests.rs"]
mod tests;
