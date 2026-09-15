use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const BUNDLED_REL: &str = "assets/cursorpacks";

const DEFAULT_REL: &str = "assets/cursors";

static BUNDLED: OnceLock<Option<PathBuf>> = OnceLock::new();
static DEFAULT_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

pub fn cursors_dir() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor")
        .join("cursors")
}

pub fn pack_dir(pack_id: &str) -> PathBuf {
    cursors_dir().join(pack_id)
}

pub fn resource_candidates(rel: &str, resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..3 {
            let Some(d) = dir else { break };
            v.push(d.join(rel));
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    if let Some(res) = resource_dir {
        v.push(res.join(rel));
    }
    v
}

pub fn bundled_root(resource_dir: Option<&Path>) -> Option<&'static Path> {
    BUNDLED
        .get_or_init(|| {
            resource_candidates(BUNDLED_REL, resource_dir)
                .into_iter()
                .find(|p| p.is_dir())
        })
        .as_deref()
}

pub fn default_pack_dir(resource_dir: Option<&Path>) -> Option<&'static Path> {
    DEFAULT_DIR
        .get_or_init(|| {
            resource_candidates(DEFAULT_REL, resource_dir)
                .into_iter()
                .find(|p| p.is_dir())
        })
        .as_deref()
}

pub fn bundled_pack_dir(id: &str) -> Option<PathBuf> {
    let dir = bundled_root(None)?.join(id);
    dir.is_dir().then_some(dir)
}

pub fn bundled_pack_dirs(resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(root) = bundled_root(resource_dir) else {
        return Vec::new();
    };
    dirs_in(root)
}

pub fn imported_pack_dirs() -> Vec<PathBuf> {
    dirs_in(&cursors_dir())
}

fn dirs_in(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

pub fn resolve_pack_dir(pack_id: &str) -> PathBuf {
    bundled_pack_dir(pack_id).unwrap_or_else(|| pack_dir(pack_id))
}

#[cfg(test)]
#[path = "packdirs_tests.rs"]
mod tests;
