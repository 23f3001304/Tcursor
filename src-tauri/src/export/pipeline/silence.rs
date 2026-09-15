use crate::export::pipeline::audio_shift_ms;
use crate::export::render::RenderMeta;
use crate::process::proc::ffcmd;
use crate::session::paths::ProjectPaths;
use anyhow::{Context, Result};
use std::path::Path;

pub const THRESHOLD_DB: f32 = -35.0;

pub const MIN_MS: u32 = 700;

pub const PAD_MS: u32 = 150;

fn field(line: &str, key: &str) -> Option<f64> {
    let rest = &line[line.find(key)? + key.len()..];
    rest.split(|c: char| c == ' ' || c == '|')
        .next()?
        .trim()
        .parse()
        .ok()
}

pub fn parse_silencedetect(stderr: &str) -> Vec<(f64, f64)> {
    let (mut out, mut open) = (Vec::new(), None::<f64>);
    for line in stderr.lines() {
        if let Some(v) = field(line, "silence_start: ") {
            open = Some(v);
        } else if let Some(v) = field(line, "silence_end: ") {
            if let Some(s) = open.take() {
                out.push((s, v));
            }
        }
    }
    if let Some(s) = open {
        out.push((s, f64::MAX));
    }
    out
}

pub fn to_clip_ms(spans_s: &[(f64, f64)], track_shift_ms: i64) -> Vec<(u32, u32)> {
    let f = |x: f64| {
        if x == f64::MAX {
            u32::MAX
        } else {
            ((x * 1000.0).round() as i64 + track_shift_ms).max(0) as u32
        }
    };
    spans_s.iter().map(|&(a, b)| (f(a), f(b))).collect()
}

pub fn intersect(a: &[(u32, u32)], b: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for &(a0, a1) in a {
        for &(b0, b1) in b {
            let (s, e) = (a0.max(b0), a1.min(b1));
            if s < e {
                out.push((s, e));
            }
        }
    }
    out.sort_unstable();
    out
}

pub fn pad_and_filter(
    spans: Vec<(u32, u32)>,
    pad_ms: u32,
    min_ms: u32,
    lo: u32,
    hi: u32,
) -> Vec<(u32, u32)> {
    spans
        .into_iter()
        .filter_map(|(s, e)| {
            let (s, e) = (
                s.clamp(lo, hi).saturating_add(pad_ms),
                e.clamp(lo, hi).saturating_sub(pad_ms),
            );
            (e > s && e - s >= min_ms).then_some((s, e))
        })
        .collect()
}

fn track_silences(track: &Path, shift_ms: i64) -> Result<Vec<(u32, u32)>> {
    let out = ffcmd("ffmpeg")
        .args(["-v", "info", "-i"])
        .arg(track)
        .args([
            "-af",
            &format!(
                "silencedetect=n={THRESHOLD_DB}dB:d={}",
                MIN_MS as f64 / 1000.0
            ),
            "-f",
            "null",
            "-",
        ])
        .output()
        .with_context(|| format!("spawn ffmpeg silencedetect on {track:?}"))?;
    Ok(to_clip_ms(
        &parse_silencedetect(&String::from_utf8_lossy(&out.stderr)),
        shift_ms,
    ))
}

pub fn detect(paths: &ProjectPaths, meta: &RenderMeta) -> Result<Vec<(u32, u32)>> {
    let shift = |t: Option<u64>| audio_shift_ms(t, meta.video_start, 0);
    let mic = paths
        .mic()
        .exists()
        .then(|| track_silences(&paths.mic(), shift(meta.tl.mic_ms)))
        .transpose()?;
    let sys = paths
        .system()
        .exists()
        .then(|| track_silences(&paths.system(), shift(meta.tl.system_ms)))
        .transpose()?;
    let spans = match (mic, sys) {
        (Some(m), Some(s)) => intersect(&m, &s),
        (Some(v), None) | (None, Some(v)) => v,
        (None, None) => vec![],
    };
    let full = (meta.video_end - meta.video_start) as u32;
    let (lo, hi) = meta.trim.resolve(full);
    Ok(pad_and_filter(spans, PAD_MS, MIN_MS, lo, hi))
}

#[tauri::command]
pub async fn detect_silences(
    folder: String,
    app: tauri::AppHandle,
) -> Result<Vec<(u32, u32)>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::export::preview::with_warm_app(&app, &folder, |c, paths| {
            detect(paths, &c.meta).map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "silence_tests.rs"]
mod tests;
