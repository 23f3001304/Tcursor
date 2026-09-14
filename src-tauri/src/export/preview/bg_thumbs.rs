// Thumbnails for the editor's background picker: every bundled wallpaper and gradient preset
// (`settings::wallpapers`) rendered through the SAME code the export uses, so a tile in the panel
// is a true 96x54 miniature of the background it applies - not a CSS approximation that drifts
// from the render. Built once per process into a `OnceLock` - and, since 2026-09-14, kept on
// disk between launches: every bundled wallpaper costs an ffmpeg decode, and the owner watched
// the panel sit on its Classic fallback for seconds after every start while 53 of them rendered.
// The first launch pays once, writes `bg_thumbs.json` to the app's cache dir, and every later
// launch reads it back in milliseconds; `prewarm` starts that on a background thread at startup
// so the editor never asks a cold cache. The key (the bundled ids and the tile size) invalidates
// the file whenever the library or the format changes.
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use crate::export::pipeline::ffio;
use crate::export::scene::background;
use crate::export::types::{Background, Rgb};
use crate::settings::wallpapers::{GRADIENT_WALLPAPERS, WALLPAPERS};

const THUMB_W: u32 = 96;
const THUMB_H: u32 = 54;

/// A gradient tile's actual stops, so the panel can apply the preset without a second copy of
/// `GRADIENT_WALLPAPERS` living in TypeScript and drifting from this one.
#[derive(Serialize, Deserialize, Clone)]
pub struct GradientStops { pub from: [u8; 3], pub mid: Option<[u8; 3]>, pub to: [u8; 3], pub angle_deg: f32 }

/// One picker tile. `kind` is the `BackgroundKind` the tile applies (`"mesh"` or `"gradient"`),
/// `id` the wallpaper id the panel writes into `BackgroundSettings.mesh`, `group` the section it
/// belongs to (a wallpaper's own `Wallpaper.group`, or `"Presets"` for the procedural gradients -
/// NOT `"Gradients"`, which is one of the wallpaper groups). `png_base64` is EMPTY when
/// the decode failed (no ffmpeg): the tile still lists, and the panel draws a plain swatch for it
/// rather than dropping a wallpaper the user can otherwise still select.
#[derive(Serialize, Deserialize, Clone)]
pub struct BackgroundThumb {
    pub id: String, pub name: String, pub kind: String, pub group: String, pub png_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub gradient: Option<GradientStops>,
}

/// What the disk copy was rendered from: the bundled ids in order plus the tile size, so a new
/// wallpaper, a renamed one or a resized tile re-renders instead of serving a stale file.
pub(crate) fn cache_key() -> String {
    let ids: Vec<&str> = WALLPAPERS.iter().map(|w| w.id).chain(GRADIENT_WALLPAPERS.iter().map(|g| g.id)).collect();
    format!("v1:{THUMB_W}x{THUMB_H}:{}", ids.join(","))
}

#[derive(Serialize, Deserialize)]
struct DiskCache { key: String, thumbs: Vec<BackgroundThumb> }

fn cache_path() -> std::path::PathBuf {
    dirs_next::cache_dir().unwrap_or_else(std::env::temp_dir).join("TCursor").join("bg_thumbs.json")
}

/// The disk copy, if it exists and was rendered from exactly this build's library.
pub(crate) fn load_disk(path: &std::path::Path, key: &str) -> Option<Vec<BackgroundThumb>> {
    let file: DiskCache = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (file.key == key && !file.thumbs.is_empty()).then_some(file.thumbs)
}

/// Best effort: a cache that cannot be written just means the next launch renders again.
pub(crate) fn save_disk(path: &std::path::Path, key: &str, thumbs: &[BackgroundThumb]) {
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(bytes) = serde_json::to_vec(&DiskCache { key: key.to_string(), thumbs: thumbs.to_vec() }) {
        let _ = std::fs::write(path, bytes);
    }
}

fn thumb(id: &str, name: &str, kind: &str, group: &str, bgra: Option<Vec<u8>>, gradient: Option<GradientStops>) -> BackgroundThumb {
    let png = bgra.and_then(|b| super::png_encode(&b, THUMB_W, THUMB_H).ok()).unwrap_or_default();
    let png_base64 = if png.is_empty() { String::new() } else { super::base64_encode(&png) };
    BackgroundThumb { id: id.into(), name: name.into(), kind: kind.into(), group: group.into(), png_base64, gradient }
}

fn rgb(c: [u8; 3]) -> Rgb { Rgb { r: c[0], g: c[1], b: c[2] } }

fn render_all() -> Vec<BackgroundThumb> {
    let walls = WALLPAPERS.iter()
        .map(|w| thumb(w.id, w.name, "mesh", w.group, ffio::decode_image_cover(w.bytes, THUMB_W, THUMB_H).ok(), None));
    let gradients = GRADIENT_WALLPAPERS.iter().map(|g| {
        let stops = GradientStops { from: g.from, mid: g.mid, to: g.to, angle_deg: g.angle_deg };
        let bg = Background::Gradient { from: rgb(g.from), mid: g.mid.map(rgb), to: rgb(g.to), angle_deg: g.angle_deg };
        thumb(g.id, g.name, "gradient", "Presets", Some(background::render(&bg, THUMB_W, THUMB_H)), Some(stops))
    });
    walls.chain(gradients).collect()
}

pub(crate) fn cached() -> &'static Vec<BackgroundThumb> {
    static CACHE: OnceLock<Vec<BackgroundThumb>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let (path, key) = (cache_path(), cache_key());
        if let Some(t) = load_disk(&path, &key) { return t; }
        let t = render_all();
        // Only a complete render is worth keeping: with no ffmpeg every wallpaper tile is empty,
        // and caching that would pin the panel's plain swatches until the file was deleted.
        if t.iter().all(|x| !x.png_base64.is_empty()) { save_disk(&path, &key, &t); }
        t
    })
}

/// Build (or load) the thumbnails off the main thread at startup, so the editor's first
/// Background panel finds them ready. `lib.rs` spawns this next to the ffmpeg prewarm.
pub fn prewarm() { let _ = cached(); }

/// Tauri command: every background preset as a base64 PNG thumbnail, in panel order (the
/// wallpapers in their own group order, then the 12 gradient presets). `async` + `spawn_blocking`
/// because the first call decodes every bundled JPEG through an ffmpeg subprocess, which would
/// otherwise freeze the window; every later call is a clone of the cached `Vec`.
#[tauri::command]
pub async fn background_thumbs() -> Result<Vec<BackgroundThumb>, String> {
    tauri::async_runtime::spawn_blocking(|| cached().clone()).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wallpaper tiles shell out for real; without an ffmpeg those assertions are skipped
    /// rather than failed (same rule `pipeline::ffio_tests` uses).
    fn ffmpeg_present() -> bool {
        crate::win::sys::proc::ffcmd("ffmpeg").arg("-version").output().is_ok()
    }

    #[test]
    fn every_preset_gets_a_tile_in_panel_order() {
        let (t, n) = (cached(), WALLPAPERS.len());
        assert_eq!(t.len(), n + GRADIENT_WALLPAPERS.len());
        assert!(t[..n].iter().all(|x| x.kind == "mesh"));
        assert!(t[n..].iter().all(|x| x.kind == "gradient" && x.group == "Presets"),
            "the procedural gradients must not share a group name with the gradient- WALLPAPER set");
        assert_eq!(t[0].id, WALLPAPERS[0].id);
        assert_eq!(t[0].group, WALLPAPERS[0].group, "a tile carries its wallpaper's own section");
        assert_eq!(t[n].id, GRADIENT_WALLPAPERS[0].id);
        for x in t { assert!(!x.name.is_empty() && !x.id.is_empty() && !x.group.is_empty()); }
    }

    #[test]
    fn a_gradient_tile_is_the_real_render_and_carries_its_stops() {
        let g = &GRADIENT_WALLPAPERS[2];
        let bg = Background::Gradient { from: rgb(g.from), mid: g.mid.map(rgb), to: rgb(g.to), angle_deg: g.angle_deg };
        let direct = thumb(g.id, g.name, "gradient", "Presets", Some(background::render(&bg, THUMB_W, THUMB_H)), None);
        let at = WALLPAPERS.len() + 2;
        assert_eq!(cached()[at].png_base64, direct.png_base64);
        assert!(!direct.png_base64.is_empty());
        // The panel applies the preset from these, so they must be the table's own values.
        let stops = cached()[at].gradient.as_ref().expect("a gradient tile carries its stops");
        assert_eq!((stops.from, stops.mid, stops.to, stops.angle_deg), (g.from, g.mid, g.to, g.angle_deg));
        assert!(cached()[0].gradient.is_none(), "a wallpaper tile has no stops");
    }

    #[test]
    fn a_failed_decode_still_lists_the_tile() {
        let t = thumb("x", "X", "mesh", "Ribbons", None, None);
        assert_eq!(t.png_base64, "", "no PNG, but the tile is still selectable");
        assert_eq!(t.id, "x");
    }

    #[test]
    fn wallpaper_tiles_decode_and_the_cache_is_built_once() {
        assert!(std::ptr::eq(cached(), cached()), "the OnceLock must hand back the same Vec");
        if !ffmpeg_present() { eprintln!("SKIPPED: no ffmpeg on PATH"); return; }
        for x in &cached()[..WALLPAPERS.len()] { assert!(!x.png_base64.is_empty(), "{} rendered no PNG", x.id); }
    }

    #[test]
    fn the_disk_copy_round_trips_and_a_stale_key_is_ignored() {
        let dir = std::env::temp_dir().join(format!("tcursor_bgthumbs_{}", std::process::id()));
        let path = dir.join("bg_thumbs.json");
        let one = vec![BackgroundThumb { id: "x".into(), name: "X".into(), kind: "mesh".into(), group: "G".into(),
            png_base64: "AAAA".into(), gradient: None }];
        save_disk(&path, "k1", &one);
        assert_eq!(load_disk(&path, "k1").map(|t| t[0].id.clone()), Some("x".to_string()));
        assert!(load_disk(&path, "k2").is_none(), "another build's key must not be served");
        assert!(cache_key().starts_with("v1:96x54:") && cache_key().contains("amber"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
