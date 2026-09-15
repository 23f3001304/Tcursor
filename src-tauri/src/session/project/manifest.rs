use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

pub const MANIFEST_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectManifest {
    pub version: u32,
    pub created_unix_ms: u64,
    pub source_w: u32,
    pub source_h: u32,
    pub app_version: String,
    #[serde(default)]
    pub preprocessed: bool,
}

impl ProjectManifest {
    pub fn new(source_w: u32, source_h: u32) -> Self {
        Self {
            version: MANIFEST_VERSION,
            created_unix_ms: now_unix_ms(),
            source_w,
            source_h,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            preprocessed: false,
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json =
            serde_json::to_vec_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &Path) -> io::Result<ProjectManifest> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn load_or_default(path: &Path) -> ProjectManifest {
        Self::load(path).unwrap_or_else(|_| ProjectManifest {
            version: MANIFEST_VERSION,
            created_unix_ms: 0,
            source_w: 0,
            source_h: 0,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            preprocessed: false,
        })
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
