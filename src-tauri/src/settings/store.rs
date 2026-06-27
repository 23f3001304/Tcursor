use std::path::PathBuf;
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
    #[test]
    fn config_path_is_under_tcursor() {
        let p = config_path();
        assert!(p.ends_with("config.json"));
        assert!(p.to_string_lossy().contains("TCursor"));
    }
}
