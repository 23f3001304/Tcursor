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
    pub fn sync(&self) -> PathBuf { self.folder.join("sync.json") }
    pub fn settings(&self) -> PathBuf { self.folder.join("settings.json") }
    pub fn actions(&self) -> PathBuf { self.folder.join("actions.json") }
    pub fn typing(&self) -> PathBuf { self.folder.join("typing.json") }
    pub fn cursor(&self) -> PathBuf { self.folder.join("cursor.json") }
    /// `folder/cursor/` - the captured OS-cursor layer (`layer.json` + one PNG per shape). A
    /// FOLDER, unlike the `cursor.json` shape track above; the two never collide.
    pub fn cursor_dir(&self) -> PathBuf { self.folder.join("cursor") }
    pub fn cursor_layer(&self) -> PathBuf { self.cursor_dir().join("layer.json") }
    pub fn cursor_png(&self, id: u32) -> PathBuf { self.cursor_dir().join(format!("{id}.png")) }
    pub fn edit(&self) -> PathBuf { self.folder.join("edit.json") }
    /// `folder/project.tcursor` - the `ProjectManifest` written at record-stop and read by
    /// `open_project` / the file-association cold-start path. The one place that filename is
    /// spelled, so both writer and readers can never disagree on it.
    pub fn manifest(&self) -> PathBuf { self.folder.join("project.tcursor") }
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
    #[test]
    fn builds_settings_snapshot_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.settings().ends_with("settings.json"));
    }
    #[test]
    fn builds_actions_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.actions().ends_with("actions.json"));
    }
    #[test]
    fn builds_typing_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.typing().ends_with("typing.json"));
    }
    #[test]
    fn builds_cursor_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.cursor().ends_with("cursor.json"));
    }
    #[test]
    fn builds_cursor_layer_paths_under_a_cursor_folder() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.cursor_dir().ends_with("cursor"));
        assert!(p.cursor_layer().ends_with("cursor/layer.json") || p.cursor_layer().ends_with("cursor\\layer.json"));
        assert!(p.cursor_png(3).ends_with("3.png"));
        // The shape track is a FILE called cursor.json - it must not collide with the folder.
        assert_ne!(p.cursor(), p.cursor_dir());
    }
    #[test]
    fn builds_edit_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.edit().ends_with("edit.json"));
    }
    #[test]
    fn builds_manifest_path() {
        let p = ProjectPaths::new(std::path::Path::new("C:/base"), "rec1");
        assert!(p.manifest().ends_with("project.tcursor"));
    }
}
