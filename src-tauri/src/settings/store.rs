use std::path::{Path, PathBuf};
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

/// `<config-dir>/TCursor/config.json` (falls back to a temp dir).
pub fn config_path() -> PathBuf {
    dirs_next::config_dir().unwrap_or_else(std::env::temp_dir).join("TCursor").join("config.json")
}

/// Load settings, defaulting on a missing or unreadable/corrupt file.
pub fn load() -> Settings { load_from(&config_path()) }

/// `load`'s actual logic, taking an explicit path - split out (mirroring `EditDoc::load`) so the
/// corrupt-preservation behavior is unit-testable against a throwaway temp file instead of the
/// real `config_path()`. A parse failure (as opposed to a missing file) preserves the bad bytes at
/// `<path>.corrupt` first, so a torn write from a crash mid-`save` doesn't silently reset every
/// setting with no recovery path.
fn load_from(path: &Path) -> Settings {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(s) => s,
            Err(e) => {
                crate::win::sys::proc::preserve_corrupt(path, &e);
                Settings::default()
            }
        },
        Err(_) => Settings::default(),
    }
}

/// The settings snapshot `start_recording` froze into the recording folder. Written once at
/// record start and never again, so it is the only trustworthy record of how the video was
/// actually captured (the doc's own `settings` are editable and drift from it).
pub fn record_snapshot(paths: &ProjectPaths) -> Settings {
    std::fs::read(paths.settings()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Whether this recording's video has the OS cursor baked into its pixels. True only for a
/// PRE-LAYER `System` recording: since the cursor became its own layer the capture is always
/// cursor-free, and the layer's presence is what says so. Missing or corrupt snapshot -> `true`
/// on a project with no layer, i.e. draw no synthetic cursor, never a double cursor.
pub fn os_cursor_in_video(paths: &ProjectPaths) -> bool {
    record_snapshot(paths).cursor.style.captures_os_cursor()
        && !crate::events::track::cursorlayer::CursorLayer::exists(paths)
}

/// Serializes and persists `s`, atomically (see `save_to`).
pub fn save(s: &Settings) -> std::io::Result<()> { save_to(&config_path(), s) }

/// `save`'s actual logic, taking an explicit path (mirroring `EditDoc::save`, and split out for
/// the same testability reason as `load_from`): the bytes land at a temp sibling first, then
/// `std::fs::rename` moves them onto `path`. Without this, `std::fs::write`'s truncate-then-write
/// left a crash or forced quit mid-save free to truncate `config.json` in place, which `load`
/// would then read back as "corrupt" and reset to defaults, discarding every
/// hotkey/theme/spotlight/audio setting with no recovery path.
fn save_to(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    let json = serde_json::to_vec_pretty(s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp = crate::win::sys::proc::tmp_sibling(path);
    if let Err(e) = std::fs::write(&tmp, &json) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::cursor::CursorStyle;

    #[test]
    fn config_path_is_under_tcursor() {
        let p = config_path();
        assert!(p.ends_with("config.json"));
        assert!(p.to_string_lossy().contains("TCursor"));
    }

    /// Write a record-time snapshot with `style` into a fresh temp folder and return its paths.
    fn snapshot_with(style: CursorStyle, tag: &str) -> ProjectPaths {
        let dir = std::env::temp_dir().join(format!("tcursor-oscur-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = ProjectPaths { folder: dir };
        let mut s = Settings::default();
        s.cursor.style = style;
        std::fs::write(paths.settings(), serde_json::to_vec(&s).unwrap()).unwrap();
        paths
    }

    #[test]
    fn os_cursor_in_video_is_derived_from_the_record_time_snapshot() {
        // Only a System-style RECORDING bakes the OS cursor into the pixels; what the editor's
        // doc says today is irrelevant, which is the whole point of reading the snapshot.
        let sys = snapshot_with(CursorStyle::System, "sys");
        assert!(os_cursor_in_video(&sys));
        let enh = snapshot_with(CursorStyle::Enhanced, "enh");
        assert!(!os_cursor_in_video(&enh), "an Enhanced recording has no baked cursor");
        let hid = snapshot_with(CursorStyle::Hidden, "hid");
        assert!(!os_cursor_in_video(&hid));
        for p in [sys, enh, hid] { let _ = std::fs::remove_dir_all(&p.folder); }
    }

    #[test]
    fn a_cursor_layer_means_the_video_is_clean_whatever_the_style_was() {
        // The 2x2 matrix. A layer exists only on a recording made AFTER the capture went
        // cursor-free, so it beats the style every time; without one the style is the only
        // evidence there is, and `System` back then meant baked pixels.
        for (style, layer, want) in [
            (CursorStyle::System, false, true),
            (CursorStyle::System, true, false),
            (CursorStyle::Enhanced, false, false),
            (CursorStyle::Enhanced, true, false),
        ] {
            let p = snapshot_with(style, &format!("matrix-{style:?}-{layer}"));
            if layer {
                crate::events::track::cursorlayer::CursorLayerBuilder::default().save(&p).unwrap();
            }
            assert_eq!(os_cursor_in_video(&p), want, "{style:?} layer={layer}");
            let _ = std::fs::remove_dir_all(&p.folder);
        }
    }

    #[test]
    fn a_missing_snapshot_reports_a_baked_cursor() {
        // Fail closed: assuming "already baked" keeps today's behavior (draw nothing) rather than
        // risking a second cursor drawn on top of a real one.
        let paths = ProjectPaths { folder: std::env::temp_dir().join("tcursor-oscur-absent") };
        assert!(os_cursor_in_video(&paths));
    }

    fn tmp_config_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tcursor-store-{tag}-{}", std::process::id())).join("config.json")
    }

    #[test]
    fn save_to_then_load_from_round_trips_and_leaves_no_tmp_sibling() {
        let path = tmp_config_path("roundtrip");
        let mut s = Settings::default();
        s.audio_offset_ms = -42;
        save_to(&path, &s).unwrap();
        assert_eq!(load_from(&path), s);
        // Only the target file exists - no leftover `.part-*` temp sibling (M3: atomic tmp+rename).
        let entries: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(entries.len(), 1, "expected only config.json, found {:?}", entries);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_to_overwrites_an_existing_file() {
        let path = tmp_config_path("overwrite");
        save_to(&path, &Settings::default()).unwrap();
        let mut s2 = Settings::default();
        s2.audio_offset_ms = 7;
        save_to(&path, &s2).unwrap();
        assert_eq!(load_from(&path), s2);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_from_a_truncated_file_returns_defaults_and_preserves_the_original_at_dot_corrupt() {
        // M3: mirrors `model_tests.rs`'s `load_on_truncated_json_returns_none_and_preserves_original_bytes`
        // - a torn write (simulated here as truncated/malformed JSON) must NOT be silently
        // destroyed by the fallback-to-defaults path.
        let path = tmp_config_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{\"clickfx\":{\"spotlight\":tr").unwrap(); // truncated mid-write
        let s = load_from(&path);
        assert_eq!(s, Settings::default(), "a corrupt config.json must fail closed to defaults");
        assert_eq!(std::fs::read(crate::win::sys::proc::corrupt_sibling(&path)).unwrap(), b"{\"clickfx\":{\"spotlight\":tr");
        assert!(!path.exists(), "the corrupt bytes must be moved OFF config.json, not left behind");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_from_a_missing_file_returns_defaults_without_touching_disk() {
        let path = tmp_config_path("missing");
        assert_eq!(load_from(&path), Settings::default());
        assert!(!path.exists() && !crate::win::sys::proc::corrupt_sibling(&path).exists());
    }
}
