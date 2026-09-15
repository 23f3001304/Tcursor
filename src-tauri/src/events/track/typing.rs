use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct TypingLog {
    pub ms: Vec<u32>,
}

impl TypingLog {
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn typing_log_round_trip() {
        let dir = std::env::temp_dir().join("tcursor_typing_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path: PathBuf = dir.join("typing.json");
        let log = TypingLog {
            ms: vec![100, 250, 500, 1200],
        };
        log.save(&path).unwrap();
        let loaded = TypingLog::load(&path);
        assert_eq!(log, loaded);
    }

    #[test]
    fn typing_log_missing_file_returns_default() {
        let path = std::path::Path::new("C:/nonexistent/path/typing.json");
        let loaded = TypingLog::load(path);
        assert_eq!(loaded, TypingLog::default());
    }
}
