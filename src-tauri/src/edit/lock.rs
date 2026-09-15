use crate::session::paths::ProjectPaths;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub(crate) fn doc_lock(p: &ProjectPaths) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut map = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    map.entry(p.folder.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_folder(tag: &str) -> ProjectPaths {
        let dir =
            std::env::temp_dir().join(format!("tcursor-doclock-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        ProjectPaths { folder: dir }
    }

    #[test]
    fn doc_lock_is_shared_per_folder_and_distinct_across_folders() {
        let (a, b) = (tmp_folder("a"), tmp_folder("b"));
        assert!(
            Arc::ptr_eq(&doc_lock(&a), &doc_lock(&a)),
            "repeated calls for the same folder share one lock"
        );
        assert!(
            !Arc::ptr_eq(&doc_lock(&a), &doc_lock(&b)),
            "unrelated projects must not contend on one lock"
        );
        let _ = std::fs::remove_dir_all(&a.folder);
        let _ = std::fs::remove_dir_all(&b.folder);
    }
}
