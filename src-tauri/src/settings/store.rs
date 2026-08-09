use std::path::PathBuf;
use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;

/// `<config-dir>/TCursor/config.json` (falls back to a temp dir).
pub fn config_path() -> PathBuf {
    dirs_next::config_dir().unwrap_or_else(std::env::temp_dir).join("TCursor").join("config.json")
}

/// Load settings, defaulting on a missing or unreadable/corrupt file.
pub fn load() -> Settings {
    std::fs::read(config_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// The settings snapshot `start_recording` froze into the recording folder. Written once at
/// record start and never again, so it is the only trustworthy record of how the video was
/// actually captured (the doc's own `settings` are editable and drift from it).
pub fn record_snapshot(paths: &ProjectPaths) -> Settings {
    std::fs::read(paths.settings()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Whether this recording's video has the OS cursor baked into its pixels. Missing or corrupt
/// snapshot -> `true`, i.e. today's behavior (draw no synthetic cursor), never a double cursor.
pub fn os_cursor_in_video(paths: &ProjectPaths) -> bool {
    record_snapshot(paths).cursor.style.captures_os_cursor()
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    let json = serde_json::to_vec_pretty(s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::CursorStyle;

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
    fn a_missing_snapshot_reports_a_baked_cursor() {
        // Fail closed: assuming "already baked" keeps today's behavior (draw nothing) rather than
        // risking a second cursor drawn on top of a real one.
        let paths = ProjectPaths { folder: std::env::temp_dir().join("tcursor-oscur-absent") };
        assert!(os_cursor_in_video(&paths));
    }
}
