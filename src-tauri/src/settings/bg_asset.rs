use crate::export::pipeline::ffio;
use crate::process::proc::ffcmd;
use serde::Serialize;
use std::path::{Path, PathBuf};

const EXTS: &[(&str, &str)] = &[
    ("png", "image"),
    ("jpg", "image"),
    ("jpeg", "image"),
    ("webp", "image"),
    ("gif", "video"),
    ("mp4", "video"),
    ("webm", "video"),
    ("mov", "video"),
];
const ACCEPTED: &str = "png, jpg, jpeg, webp, gif, mp4, webm or mov";

const THUMB_W: u32 = 320;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BackgroundAssetInfo {
    pub rel_path: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub duration_ms: Option<u64>,
}

pub fn asset_kind_for(ext: &str) -> Option<&'static str> {
    let e = ext.to_ascii_lowercase();
    EXTS.iter().find(|(x, _)| *x == e).map(|(_, k)| *k)
}

pub fn unique_name(dir: &Path, file_name: &str) -> String {
    if !dir.join(file_name).exists() {
        return file_name.to_string();
    }
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) => (&file_name[..i], &file_name[i..]),
        None => (file_name, ""),
    };
    (2..)
        .map(|n| format!("{stem}_{n}{ext}"))
        .find(|c| !dir.join(c).exists())
        .unwrap()
}

fn resolve_rel(project_dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return None;
    }
    if !p
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    Some(project_dir.join(p))
}

pub fn asset_path(project_dir: &Path, rel: &str) -> Option<PathBuf> {
    resolve_rel(project_dir, rel).filter(|p| p.is_file())
}

pub fn thumb_rel(rel: &str) -> String {
    match rel.rfind('/') {
        Some(i) => format!("{}/.thumbs/{}.jpg", &rel[..i], &rel[i + 1..]),
        None => format!(".thumbs/{rel}.jpg"),
    }
}

pub fn probe_asset(file: &Path, kind: &str) -> (u32, u32, Option<u64>) {
    let (w, h) = ffio::probe_dims(file).unwrap_or((0, 0));
    let dur = (kind == "video")
        .then(|| {
            ffio::probe_duration(file)
                .ok()
                .filter(|s| *s > 0.0)
                .map(|s| (s * 1000.0).round() as u64)
        })
        .flatten();
    (w, h, dur)
}

pub fn write_thumb(src: &Path, dest: &Path) -> bool {
    if let Some(d) = dest.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let ok = ffcmd("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(src)
        .args([
            "-frames:v",
            "1",
            "-vf",
            &format!("scale={THUMB_W}:-2"),
            "-q:v",
            "3",
        ])
        .arg(dest)
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ok && dest.is_file()
}

pub fn import_blocking(project_dir: &Path, src: &Path) -> Result<BackgroundAssetInfo, String> {
    if !src.is_file() {
        return Err(format!("not a file: {}", src.display()));
    }
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
    let kind = asset_kind_for(ext)
        .ok_or_else(|| format!("unsupported background file type \".{ext}\" - use {ACCEPTED}"))?;
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("background")
        .to_string();
    let dir = project_dir.join("background");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create background folder: {e}"))?;
    let name = unique_name(&dir, &name);
    std::fs::copy(src, dir.join(&name)).map_err(|e| format!("copy background asset: {e}"))?;
    let rel = format!("background/{name}");
    let dest = dir.join(&name);
    write_thumb(&dest, &project_dir.join(thumb_rel(&rel)));
    let (width, height, duration_ms) = probe_asset(&dest, kind);
    Ok(BackgroundAssetInfo {
        rel_path: rel,
        kind: kind.to_string(),
        width,
        height,
        duration_ms,
    })
}

pub fn info_blocking(project_dir: &Path, rel: &str) -> Option<BackgroundAssetInfo> {
    let file = asset_path(project_dir, rel)?;
    let kind = asset_kind_for(file.extension().and_then(|e| e.to_str()).unwrap_or(""))?;
    let thumb = project_dir.join(thumb_rel(rel));
    if !thumb.is_file() {
        write_thumb(&file, &thumb);
    }
    let (width, height, duration_ms) = probe_asset(&file, kind);
    Some(BackgroundAssetInfo {
        rel_path: rel.to_string(),
        kind: kind.to_string(),
        width,
        height,
        duration_ms,
    })
}

pub fn remove_blocking(project_dir: &Path, rel: &str) -> Result<(), String> {
    let file =
        resolve_rel(project_dir, rel).ok_or_else(|| format!("not a project asset: {rel}"))?;
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(project_dir.join(thumb_rel(rel)));
    Ok(())
}

#[tauri::command]
pub async fn import_background_asset(
    project_dir: String,
    src_path: String,
) -> Result<BackgroundAssetInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        import_blocking(Path::new(&project_dir), Path::new(&src_path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn background_asset_info(
    project_dir: String,
    rel_path: String,
) -> Result<Option<BackgroundAssetInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(info_blocking(Path::new(&project_dir), &rel_path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn remove_background_asset(project_dir: String, rel_path: String) -> Result<(), String> {
    remove_blocking(Path::new(&project_dir), &rel_path)
}

#[cfg(test)]
#[path = "bg_asset_tests.rs"]
mod tests;
