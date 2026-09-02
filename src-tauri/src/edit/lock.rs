// Per-project-folder lock guarding every write `edit.json` can receive, wherever it originates -
// split into its own file (bug-sweep-2 Task 7 round 2) so `edit::commands` and `edit::seed` share
// exactly ONE lock registry instead of each rolling its own.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use crate::session::paths::ProjectPaths;

/// One lock per project folder (H3). Without it, any two of `edit::commands::apply_edit_op`'s
/// read-modify-write, `save_edit`'s blind write, and `edit::seed::load_or_seed`'s OWN internal
/// seed/migrate/lift write have no ordering guarantee against each other - whichever reaches disk
/// LAST wins even if it read a now-stale copy, silently discarding whatever another writer just
/// wrote (`EditDoc::save`'s tmp+rename is atomic w.r.t. TORN files, but says nothing about WHICH
/// document wins a race).
///
/// **Round 2 fix:** `load_or_seed` (`edit::seed`) is the ONE place every caller in the tree
/// reaches `edit.json` through - not just `edit::commands`' three IPC entry points. Round 1 of
/// this fix only locked those three, leaving `load_or_seed`'s own seed/migrate/lift write
/// unlocked everywhere else it's called from: `ai::commands::build_plan`,
/// `export::preview::session`'s warm-cache `reuse`/`build`, `export::render::{FrameRenderer::new,
/// render_edit::EditState::load}`, `export::cursor::cursorpreview::cursor_sprites`,
/// `export::preview::preprocess::run`, `export::preview::thumbs::preview_audio_shifts`. Taking
/// this lock INSIDE `load_or_seed` itself (see `seed.rs`) is what actually covers all of them;
/// `edit::commands`' three commands still take it too, for the parts of their own work
/// `load_or_seed` doesn't cover (`apply_edit_op`'s `apply` + save, `save_edit`'s save) - `apply_edit_op`
/// takes it ONCE across both `load_or_seed`'s locked core (`seed::load_or_seed_locked`) and its
/// own write, never twice (`std::sync::Mutex` is NOT reentrant - see `seed.rs`'s module doc for
/// why `get_edit`/`apply_edit_op` route around calling the self-locking `load_or_seed` while
/// already holding this lock).
///
/// **Lock ordering vs `export::preview::session::WarmSlot`:** `WarmSlot::with` holds its `gate`
/// mutex across the whole `reuse`/`make`/`work` call (never its `cell` mutex, which is only ever
/// held to swap the cached entry in/out - see `session.rs`'s own module doc). `reuse`/`build`
/// call `load_or_seed` while `gate` is held, so the only ordering this lock ever participates in
/// is `gate -> doc_lock`. The reverse (`doc_lock -> gate`) never occurs: nothing reachable from
/// inside this lock's critical section (`edit::seed`, `edit::migrate`, `edit::ops::effects`,
/// `EditDoc::save`) ever touches `WarmSlot`, `PreviewSession`, or
/// `win::sys::proc::generate_once`'s lock (confirmed by grep - none of those types/functions are
/// referenced anywhere under `edit::`). `generate_once`'s callers (`thumbs.rs`, `preview_track.rs`)
/// all call `load_or_seed`-touching helpers (`load_or_seed` itself, `preview_audio_shifts`)
/// *before* taking `generate_once`'s lock, never inside its closure, so there is no
/// `generate_once -> doc_lock` ordering to worry about either.
///
/// Per-folder rather than one process-global lock so unrelated open projects never contend.
pub(crate) fn doc_lock(p: &ProjectPaths) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut map = LOCKS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    map.entry(p.folder.clone()).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_folder(tag: &str) -> ProjectPaths {
        let dir = std::env::temp_dir().join(format!("tcursor-doclock-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        ProjectPaths { folder: dir }
    }

    #[test]
    fn doc_lock_is_shared_per_folder_and_distinct_across_folders() {
        let (a, b) = (tmp_folder("a"), tmp_folder("b"));
        assert!(Arc::ptr_eq(&doc_lock(&a), &doc_lock(&a)), "repeated calls for the same folder share one lock");
        assert!(!Arc::ptr_eq(&doc_lock(&a), &doc_lock(&b)), "unrelated projects must not contend on one lock");
        let _ = std::fs::remove_dir_all(&a.folder);
        let _ = std::fs::remove_dir_all(&b.folder);
    }
}
