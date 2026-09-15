use crate::ai::frames::sample::FrameAt;
use crate::process::proc::ffcmd_bg;
use std::path::Path;
use std::process::Stdio;

pub const LONG_EDGE: u32 = 512;

pub const JPEG_QV: u32 = 5;

pub fn jpeg_at(video: &Path, t_ms: u32, long_edge: u32) -> Result<Vec<u8>, String> {
    let ss = format!("{:.3}", t_ms as f64 / 1000.0);
    let vf = format!("scale={long_edge}:{long_edge}:force_original_aspect_ratio=decrease");
    let out = ffcmd_bg("ffmpeg")
        .args(["-v", "error", "-ss", &ss, "-i"])
        .arg(video)
        .args([
            "-frames:v",
            "1",
            "-vf",
            &vf,
            "-f",
            "mjpeg",
            "-q:v",
            &JPEG_QV.to_string(),
            "-",
        ])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("spawn ffmpeg (frame at {t_ms}ms): {e}"))?;
    let sized = jpeg_dims(&out.stdout).map_or(false, |(w, h)| w > 0 && h > 0);
    if sized {
        Ok(out.stdout)
    } else {
        Err(format!("no frame at {t_ms}ms"))
    }
}

pub fn jpegs_at(folder: &str, times: &[FrameAt], long_edge: u32) -> Vec<(FrameAt, Vec<u8>)> {
    let proxy = match crate::export::preview::preview_track::ensure_proxy_blocking(
        folder.to_string(),
        crate::export::preview::preprocess::DEFAULT_PROXY_HEIGHT,
    ) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let path = Path::new(&proxy);
    times
        .iter()
        .filter_map(|&at| {
            jpeg_at(path, at.t_ms, long_edge)
                .ok()
                .map(|bytes| (at, bytes))
        })
        .collect()
}

pub(crate) fn jpeg_dims(jpg: &[u8]) -> Option<(u32, u32)> {
    if jpg.len() < 4 || jpg[0] != 0xFF || jpg[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 3 < jpg.len() {
        if jpg[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = jpg[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        if matches!(marker, 0x01 | 0xD0..=0xD9) {
            i += 2;
            continue;
        }
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            if i + 9 > jpg.len() {
                return None;
            }
            let h = u16::from_be_bytes([jpg[i + 5], jpg[i + 6]]) as u32;
            let w = u16::from_be_bytes([jpg[i + 7], jpg[i + 8]]) as u32;
            return Some((w, h));
        }
        i += 2 + u16::from_be_bytes([jpg[i + 2], jpg[i + 3]]).max(2) as usize;
    }
    None
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
