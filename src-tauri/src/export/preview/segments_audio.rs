use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::process::proc::{ffcmd_bg, tmp_sibling};
use crate::session::paths::ProjectPaths;
use crate::session::sync::SyncLog;

#[derive(Clone, Debug, PartialEq)]
pub struct MicPart {
    pub path: PathBuf,
    pub delay_ms: u64,
}

fn layout(channels: u16) -> String {
    match channels {
        1 => "mono".into(),
        2 => "stereo".into(),
        n => format!("{n}c"),
    }
}

pub fn merge_args(parts: &[MicPart], rate: u32, channels: u16, out: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    for p in parts {
        args.push("-i".into());
        args.push(p.path.as_os_str().to_os_string());
    }
    let (lay, n) = (layout(channels), parts.len());
    let branches: Vec<String> = parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let delay = if p.delay_ms == 0 {
                String::new()
            } else {
                let each: Vec<String> = (0..channels.max(1))
                    .map(|_| p.delay_ms.to_string())
                    .collect();
                format!(",adelay={}", each.join("|"))
            };
            let label = if n == 1 {
                "[a]".to_string()
            } else {
                format!("[s{i}]")
            };
            format!("[{i}:a]aresample={rate},aformat=channel_layouts={lay}{delay}{label}")
        })
        .collect();
    let mut filter = branches.join(";");
    if n > 1 {
        let ins: String = (0..n).map(|i| format!("[s{i}]")).collect();
        filter.push_str(&format!(
            ";{ins}amix=inputs={n}:normalize=0:dropout_transition=0[a]"
        ));
    }
    args.extend(
        [
            "-filter_complex",
            &filter,
            "-map",
            "[a]",
            "-c:a",
            "pcm_s16le",
        ]
        .map(OsString::from),
    );
    args.push(out.as_os_str().to_os_string());
    args
}

fn parts_of(paths: &ProjectPaths, sync: &SyncLog) -> Vec<MicPart> {
    let origin = sync
        .mic_ms
        .or_else(|| sync.frames.first().copied())
        .unwrap_or(0);
    let mut parts = Vec::new();
    if paths.mic().exists() {
        parts.push(MicPart {
            path: paths.mic(),
            delay_ms: 0,
        });
    }
    for s in &sync.mic_segments {
        let path = paths.folder.join(&s.path);
        if path.exists() {
            parts.push(MicPart {
                path,
                delay_ms: s.start_ms.saturating_sub(origin),
            });
        }
    }
    parts
}

fn wav_spec(path: &Path) -> Result<(u32, u16), String> {
    let spec = hound::WavReader::open(path)
        .map_err(|e| format!("read {path:?}: {e}"))?
        .spec();
    Ok((spec.sample_rate, spec.channels))
}

fn run(args: &[OsString]) -> Result<(), String> {
    let status = ffcmd_bg("ffmpeg")
        .args(["-y", "-v", "error"])
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("spawn ffmpeg (mic merge): {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg mic merge exited with {status}"))
    }
}

pub fn merge_mic_segments(paths: &ProjectPaths, sync: &SyncLog) -> Result<(), String> {
    if sync.mic_segments.is_empty() {
        return Ok(());
    }
    let parts = parts_of(paths, sync);
    let first = match parts.first() {
        Some(p) => p,
        None => return Ok(()),
    };
    if parts.len() == 1 && first.delay_ms == 0 && first.path == paths.mic() {
        return Ok(());
    }
    let (rate, channels) = wav_spec(&first.path)?;
    let (out, tmp) = (paths.mic(), tmp_sibling(&paths.mic()));
    if let Err(e) = run(&merge_args(&parts, rate, channels, &tmp)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let kept = paths.folder.join("segments");
    std::fs::create_dir_all(&kept).map_err(|e| format!("create {kept:?}: {e}"))?;
    for p in parts.iter().map(|p| &p.path) {
        if let Some(n) = p.file_name() {
            let _ = std::fs::rename(p, kept.join(n));
        }
    }
    std::fs::rename(&tmp, &out).map_err(|e| {
        let _ = std::fs::rename(kept.join("mic.wav"), &out);
        format!("rename {tmp:?} -> {out:?}: {e}")
    })
}

#[cfg(test)]
#[path = "segments_audio_tests.rs"]
mod tests;
