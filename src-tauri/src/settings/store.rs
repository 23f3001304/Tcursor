use crate::session::paths::ProjectPaths;
use crate::settings::model::Settings;
use std::path::{Path, PathBuf};

pub fn config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor")
        .join("config.json")
}

pub fn load() -> Settings {
    load_from(&config_path())
}

fn load_from(path: &Path) -> Settings {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(s) => s,
            Err(e) => {
                crate::process::proc::preserve_corrupt(path, &e);
                Settings::default()
            }
        },
        Err(_) => Settings::default(),
    }
}

pub fn record_snapshot(paths: &ProjectPaths) -> Settings {
    std::fs::read(paths.settings())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn os_cursor_in_video(paths: &ProjectPaths) -> bool {
    record_snapshot(paths).cursor.style.captures_os_cursor()
        && !crate::events::track::cursorlayer::CursorLayer::exists(paths)
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    save_to(&config_path(), s)
}

fn save_to(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp = crate::process::proc::tmp_sibling(path);
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
        let sys = snapshot_with(CursorStyle::System, "sys");
        assert!(os_cursor_in_video(&sys));
        let enh = snapshot_with(CursorStyle::Enhanced, "enh");
        assert!(
            !os_cursor_in_video(&enh),
            "an Enhanced recording has no baked cursor"
        );
        let hid = snapshot_with(CursorStyle::Hidden, "hid");
        assert!(!os_cursor_in_video(&hid));
        for p in [sys, enh, hid] {
            let _ = std::fs::remove_dir_all(&p.folder);
        }
    }

    #[test]
    fn a_cursor_layer_means_the_video_is_clean_whatever_the_style_was() {
        for (style, layer, want) in [
            (CursorStyle::System, false, true),
            (CursorStyle::System, true, false),
            (CursorStyle::Enhanced, false, false),
            (CursorStyle::Enhanced, true, false),
        ] {
            let p = snapshot_with(style, &format!("matrix-{style:?}-{layer}"));
            if layer {
                crate::events::track::cursorlayer::CursorLayerBuilder::default()
                    .save(&p)
                    .unwrap();
            }
            assert_eq!(os_cursor_in_video(&p), want, "{style:?} layer={layer}");
            let _ = std::fs::remove_dir_all(&p.folder);
        }
    }

    #[test]
    fn a_missing_snapshot_reports_a_baked_cursor() {
        let paths = ProjectPaths {
            folder: std::env::temp_dir().join("tcursor-oscur-absent"),
        };
        assert!(os_cursor_in_video(&paths));
    }

    fn tmp_config_path(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("tcursor-store-{tag}-{}", std::process::id()))
            .join("config.json")
    }

    #[test]
    fn save_to_then_load_from_round_trips_and_leaves_no_tmp_sibling() {
        let path = tmp_config_path("roundtrip");
        let mut s = Settings::default();
        s.audio_offset_ms = -42;
        save_to(&path, &s).unwrap();
        assert_eq!(load_from(&path), s);
        let entries: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(
            entries.len(),
            1,
            "expected only config.json, found {:?}",
            entries
        );
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
        std::fs::write(&path, b"{\"clickfx\":{\"spotlight\":tr").unwrap();
        let s = load_from(&path);
        assert_eq!(
            s,
            Settings::default(),
            "a corrupt config.json must fail closed to defaults"
        );
        assert_eq!(
            std::fs::read(crate::process::proc::corrupt_sibling(&path)).unwrap(),
            b"{\"clickfx\":{\"spotlight\":tr"
        );
        assert!(
            !path.exists(),
            "the corrupt bytes must be moved OFF config.json, not left behind"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_from_a_missing_file_returns_defaults_without_touching_disk() {
        let path = tmp_config_path("missing");
        assert_eq!(load_from(&path), Settings::default());
        assert!(!path.exists() && !crate::process::proc::corrupt_sibling(&path).exists());
    }
}
