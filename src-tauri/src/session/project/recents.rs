use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_RECENTS: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecentProject {
    pub folder: String,
    pub name: String,
    pub opened_unix_ms: u64,
}

fn recents_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor")
        .join("recents.json")
}

pub fn list() -> Vec<RecentProject> {
    std::fs::read(recents_path())
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<RecentProject>>(&b).ok())
        .unwrap_or_default()
}

pub fn touch(folder: &str) {
    let name = std::path::Path::new(folder)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(folder)
        .to_string();

    let mut entries = list();
    entries.retain(|r| r.folder != folder);
    entries.insert(
        0,
        RecentProject {
            folder: folder.to_string(),
            name,
            opened_unix_ms: now_unix_ms(),
        },
    );
    entries.truncate(MAX_RECENTS);

    let path = recents_path();
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    if let Ok(json) = serde_json::to_vec_pretty(&entries) {
        let _ = std::fs::write(path, json);
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recents_path_is_under_tcursor() {
        let p = recents_path();
        assert!(p.ends_with("recents.json"));
        assert!(p.to_string_lossy().contains("TCursor"));
    }

    #[test]
    fn touch_moves_an_existing_folder_to_the_front_without_duplicating_it() {
        let mut entries = vec![
            RecentProject {
                folder: "a".into(),
                name: "a".into(),
                opened_unix_ms: 1,
            },
            RecentProject {
                folder: "b".into(),
                name: "b".into(),
                opened_unix_ms: 2,
            },
        ];
        entries.retain(|r| r.folder != "a");
        entries.insert(
            0,
            RecentProject {
                folder: "a".into(),
                name: "a".into(),
                opened_unix_ms: 3,
            },
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].folder, "a");
        assert_eq!(entries[0].opened_unix_ms, 3);
    }

    #[test]
    fn truncates_to_max_recents() {
        let mut entries: Vec<RecentProject> = (0..(MAX_RECENTS + 5))
            .map(|i| RecentProject {
                folder: i.to_string(),
                name: i.to_string(),
                opened_unix_ms: i as u64,
            })
            .collect();
        entries.truncate(MAX_RECENTS);
        assert_eq!(entries.len(), MAX_RECENTS);
    }
}
