use std::path::{Path, PathBuf};

pub struct ProjectPaths { pub folder: PathBuf }

impl ProjectPaths {
    pub fn new(base: &Path, name: &str) -> Self {
        Self { folder: base.join(name) }
    }
    pub fn video(&self) -> PathBuf { self.folder.join("video.mp4") }
    pub fn mic(&self) -> PathBuf { self.folder.join("mic.wav") }
    pub fn events(&self) -> PathBuf { self.folder.join("events.json") }
    pub fn system(&self) -> PathBuf { self.folder.join("system.wav") }
    pub fn webcam(&self) -> PathBuf { self.folder.join("webcam.webm") }
    pub fn ensure(&self) -> std::io::Result<()> { std::fs::create_dir_all(&self.folder) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn builds_system_and_webcam_paths() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.system().ends_with("system.wav"));
        assert!(p.webcam().ends_with("webcam.webm"));
    }
    #[test]
    fn builds_track_paths_under_named_folder() {
        let p = ProjectPaths::new(Path::new("C:/base"), "rec1");
        assert!(p.folder.ends_with("rec1"));
        assert!(p.video().ends_with("video.mp4"));
        assert!(p.mic().ends_with("mic.wav"));
        assert!(p.events().ends_with("events.json"));
    }
}
