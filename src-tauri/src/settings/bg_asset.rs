// The user's own background file: import it INTO the project, describe it, remove it.
//
// Everything here exists to keep a project portable. The file is copied into
// `<project>/background/` and `BackgroundSettings.asset` stores only `background/<name>` - a
// relative, forward-slashed path - so moving or copying the project folder moves the background
// with it. Nothing outside the project is ever referenced, and `resolve_rel` refuses to resolve a
// stored path that is absolute or climbs out with `..`, so a hand-edited `edit.json` cannot turn
// the background into a file-read primitive.
//
// Probing and thumbnailing both go through the bundled ffmpeg/ffprobe (`ffio`), which reads every
// accepted format; there is no image-decoding crate in this build and this file does not add one.
use std::path::{Path, PathBuf};
use serde::Serialize;
use crate::export::pipeline::ffio;
use crate::win::sys::proc::ffcmd;

/// Accepted extensions and the `BackgroundKind` each maps to. A GIF is a VIDEO: one decode path
/// for both, so the export never grows a second animation subsystem.
const EXTS: &[(&str, &str)] = &[
    ("png", "image"), ("jpg", "image"), ("jpeg", "image"), ("webp", "image"),
    ("gif", "video"), ("mp4", "video"), ("webm", "video"), ("mov", "video"),
];
const ACCEPTED: &str = "png, jpg, jpeg, webp, gif, mp4, webm or mov";
/// Thumbnail width; the height follows the source's aspect (`-2` keeps it even for the encoder).
const THUMB_W: u32 = 320;

/// What the panel needs to show for an imported background: where it lives (relative), which kind
/// it is, its pixel size, and - for a video - how long it loops.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BackgroundAssetInfo {
    pub rel_path: String, pub kind: String,
    pub width: u32, pub height: u32, pub duration_ms: Option<u64>,
}

/// `"image"` / `"video"` for an accepted extension (case-insensitive), `None` for anything else.
pub fn asset_kind_for(ext: &str) -> Option<&'static str> {
    let e = ext.to_ascii_lowercase();
    EXTS.iter().find(|(x, _)| *x == e).map(|(_, k)| *k)
}

/// `file_name`, or the first of `<stem>_2.<ext>`, `<stem>_3.<ext>`, ... that `dir` does not
/// already hold. Importing the same file twice must never overwrite the copy an earlier project
/// state still points at.
pub fn unique_name(dir: &Path, file_name: &str) -> String {
    if !dir.join(file_name).exists() { return file_name.to_string(); }
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) => (&file_name[..i], &file_name[i..]), // ext carries its own dot
        None => (file_name, ""),
    };
    (2..).map(|n| format!("{stem}_{n}{ext}")).find(|c| !dir.join(c).exists()).unwrap()
}

/// A stored relative path as an absolute one, WITHOUT checking that it exists. `None` unless the
/// path is relative, non-empty and made only of plain names - no `..`, no root, no drive prefix.
fn resolve_rel(project_dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() { return None; }
    let p = Path::new(rel);
    if p.is_absolute() { return None; }
    if !p.components().all(|c| matches!(c, std::path::Component::Normal(_))) { return None; }
    Some(project_dir.join(p))
}

/// `resolve_rel` plus "and the file is really there". `None` is the everyday case of a project
/// whose asset was deleted or was moved without its `background/` folder - callers fall back to
/// the base wallpaper rather than failing.
pub fn asset_path(project_dir: &Path, rel: &str) -> Option<PathBuf> {
    resolve_rel(project_dir, rel).filter(|p| p.is_file())
}

/// Where an asset's thumbnail lives, derived from the asset's own relative path so the pair can
/// never drift: `background/clip.mp4` -> `background/.thumbs/clip.mp4.jpg`. The full file name
/// (extension included) is kept, so `a.png` and `a.mp4` get different thumbs.
pub fn thumb_rel(rel: &str) -> String {
    match rel.rfind('/') {
        Some(i) => format!("{}/.thumbs/{}.jpg", &rel[..i], &rel[i + 1..]),
        None => format!(".thumbs/{rel}.jpg"),
    }
}

/// Pixel size and (video only) duration in ms, via ffprobe. A probe that fails reports zeros
/// rather than an error: the import still succeeded, and the card simply has less to say.
pub fn probe_asset(file: &Path, kind: &str) -> (u32, u32, Option<u64>) {
    let (w, h) = ffio::probe_dims(file).unwrap_or((0, 0));
    let dur = (kind == "video")
        .then(|| ffio::probe_duration(file).ok().filter(|s| *s > 0.0).map(|s| (s * 1000.0).round() as u64))
        .flatten();
    (w, h, dur)
}

/// Write a `THUMB_W`-wide JPEG of `src`'s first frame to `dest`. `false` if ffmpeg is absent or
/// refused the file - a missing thumbnail is cosmetic, never fatal.
pub fn write_thumb(src: &Path, dest: &Path) -> bool {
    if let Some(d) = dest.parent() { let _ = std::fs::create_dir_all(d); }
    let ok = ffcmd("ffmpeg")
        .args(["-v", "error", "-y", "-i"]).arg(src)
        .args(["-frames:v", "1", "-vf", &format!("scale={THUMB_W}:-2"), "-q:v", "3"]).arg(dest)
        .stderr(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false);
    ok && dest.is_file()
}

/// Copy `src` into `<project>/background/`, thumbnail it, and describe it. Split out of the
/// command so it can be tested against two temp folders.
pub fn import_blocking(project_dir: &Path, src: &Path) -> Result<BackgroundAssetInfo, String> {
    if !src.is_file() { return Err(format!("not a file: {}", src.display())); }
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
    let kind = asset_kind_for(ext)
        .ok_or_else(|| format!("unsupported background file type \".{ext}\" - use {ACCEPTED}"))?;
    let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("background").to_string();
    let dir = project_dir.join("background");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create background folder: {e}"))?;
    let name = unique_name(&dir, &name);
    std::fs::copy(src, dir.join(&name)).map_err(|e| format!("copy background asset: {e}"))?;
    let rel = format!("background/{name}");
    let dest = dir.join(&name);
    write_thumb(&dest, &project_dir.join(thumb_rel(&rel)));
    let (width, height, duration_ms) = probe_asset(&dest, kind);
    Ok(BackgroundAssetInfo { rel_path: rel, kind: kind.to_string(), width, height, duration_ms })
}

/// Describe an already-imported asset, re-creating a thumbnail that went missing. `None` when the
/// file is gone (the card says so instead of showing a stale name).
pub fn info_blocking(project_dir: &Path, rel: &str) -> Option<BackgroundAssetInfo> {
    let file = asset_path(project_dir, rel)?;
    let kind = asset_kind_for(file.extension().and_then(|e| e.to_str()).unwrap_or(""))?;
    let thumb = project_dir.join(thumb_rel(rel));
    if !thumb.is_file() { write_thumb(&file, &thumb); }
    let (width, height, duration_ms) = probe_asset(&file, kind);
    Some(BackgroundAssetInfo { rel_path: rel.to_string(), kind: kind.to_string(), width, height, duration_ms })
}

/// Delete an imported asset and its thumbnail. Idempotent (a second Remove, or a file already
/// gone, is success), and bounded by the same path rules as every other resolve.
pub fn remove_blocking(project_dir: &Path, rel: &str) -> Result<(), String> {
    let file = resolve_rel(project_dir, rel).ok_or_else(|| format!("not a project asset: {rel}"))?;
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(project_dir.join(thumb_rel(rel)));
    Ok(())
}

/// Tauri command: copy the user's chosen file into the project and hand back its relative path.
/// `async` + `spawn_blocking` - it copies a file that can be gigabytes and shells out twice.
#[tauri::command]
pub async fn import_background_asset(project_dir: String, src_path: String) -> Result<BackgroundAssetInfo, String> {
    tauri::async_runtime::spawn_blocking(move || import_blocking(Path::new(&project_dir), Path::new(&src_path)))
        .await.map_err(|e| e.to_string())?
}

/// Tauri command: what the panel shows for the asset already named in `edit.json`.
#[tauri::command]
pub async fn background_asset_info(project_dir: String, rel_path: String) -> Result<Option<BackgroundAssetInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(info_blocking(Path::new(&project_dir), &rel_path)))
        .await.map_err(|e| e.to_string())?
}

/// Tauri command: delete the asset + thumb. It does NOT touch `edit.json` - the doc lives in the
/// frontend and is written by `save_edit`, so the panel clears `asset` in the same save that
/// follows this call; a write from here would be silently overwritten by the next one.
#[tauri::command]
pub fn remove_background_asset(project_dir: String, rel_path: String) -> Result<(), String> {
    remove_blocking(Path::new(&project_dir), &rel_path)
}

#[cfg(test)]
#[path = "bg_asset_tests.rs"]
mod tests;
