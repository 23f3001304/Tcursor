// `project.tcursor`: a small JSON manifest written into the project folder (never a zip/copy of
// the multi-GB video - the folder itself IS the project). Written once, `preprocessed=false`, at
// `stop_recording`; sub-project 5 will flip `preprocessed` after generating proxies/thumbs/
// waveform ahead of time so the editor can skip regenerating them. Back-compat is the point of
// `load_or_default`: recordings made before this feature have no manifest at all, and every
// caller must still be able to open them.
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Bumped only if the manifest's shape changes in a way readers must branch on.
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
    /// A fresh manifest for a just-finished recording: current time, this build's crate version,
    /// `preprocessed: false` (nothing has pre-generated proxies/thumbs for it yet).
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
        let json = serde_json::to_vec_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &Path) -> io::Result<ProjectManifest> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    /// `Self::load(path)`, or a fresh "unknown source" default if the manifest is missing,
    /// unreadable, or corrupt. Existing recordings made before this feature had no
    /// `project.tcursor` on disk at all, so callers (`open_project`, the file-association
    /// cold-start path) must still be able to proceed with them - a synthesized manifest with
    /// `source_w/h = 0` is a clear "unknown" signal, not a lie about the real capture size.
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
