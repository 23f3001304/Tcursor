use crate::edit::model::{EditDoc};
use crate::edit::ops::api::EditOp;
use crate::session::paths::ProjectPaths;

fn paths(folder: &str) -> ProjectPaths {
    ProjectPaths { folder: std::path::PathBuf::from(folder) }
}

/// `async` + `spawn_blocking`: on a project that was never preprocessed (a legacy recording, or
/// one whose preprocess pass failed) `load_or_seed` falls into `seed::build_default`, which
/// gzip-decodes the whole event log, runs autozoom over every mouse sample and spawns up to two
/// `ffprobe` subprocesses to rebuild the timeline. This is the FIRST call the editor makes on
/// mount, so as a sync command that seed froze the window on the project-open path.
///
/// No explicit lock here - `seed::load_or_seed` is self-locking (`edit::lock::doc_lock`, taken
/// and released internally for the span of any write it makes) and this command does nothing to
/// `edit.json` beyond what `load_or_seed` itself already does, so delegating fully is correct and
/// avoids a redundant (and, since the lock is non-reentrant, DEADLOCKING) second acquisition.
#[tauri::command]
pub async fn get_edit(folder: String) -> Result<EditDoc, String> {
    tauri::async_runtime::spawn_blocking(move || crate::edit::seed::load_or_seed(&paths(&folder)))
        .await
        .map_err(|e| e.to_string())
}

/// One lock acquisition covers BOTH `load_or_seed`'s possible seed/migrate/lift write AND this
/// op's own apply+save (H3, bug-sweep-2 Task 7 round 2) - never two separate acquisitions.
/// `edit::lock::doc_lock` is a plain `std::sync::Mutex`, NOT reentrant, so calling the
/// self-locking `seed::load_or_seed` from inside an already-held lock would deadlock; this
/// precomputes the same ffprobe-backed inputs `load_or_seed` would (unlocked, via
/// `seed_lock::derive_seed_inputs`), then does everything else - the seed/migrate/lift decision
/// AND this command's own op+save - under one lock, via `seed_lock::load_or_seed_locked`.
#[tauri::command]
pub fn apply_edit_op(folder: String, op: EditOp) -> Result<EditDoc, String> {
    let p = paths(&folder);
    let unlocked = EditDoc::load(&p.edit());
    let (precomputed_default, shift, true_dur) = crate::edit::seed_lock::derive_seed_inputs(&p, &unlocked);
    let lock = crate::edit::lock::doc_lock(&p);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut doc = crate::edit::seed_lock::load_or_seed_locked(&p, precomputed_default, shift, true_dur);
    crate::edit::ops::api::apply(&mut doc, op);
    doc.save(&p.edit()).map_err(|e| e.to_string())?;
    Ok(doc)
}

/// Takes `edit::lock::doc_lock` directly - `save_edit` is a blind overwrite of the caller-supplied
/// `doc` and never calls `load_or_seed`, so there's no double-locking risk here (unlike
/// `apply_edit_op` above). Still needs the SAME per-folder lock as every other writer so its write
/// can't land torn between another command's load and save.
#[tauri::command]
pub fn save_edit(folder: String, doc: EditDoc) -> Result<(), String> {
    let p = paths(&folder);
    let lock = crate::edit::lock::doc_lock(&p);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    doc.save(&p.edit()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_folder(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("tcursor-editcmd-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().into_owned()
    }

    /// H3 regression: many concurrent `apply_edit_op` calls against the SAME project must not
    /// lose each other's writes. Without the shared per-folder lock serializing the
    /// read-modify-write, two threads racing `load_or_seed_locked -> apply -> save` on the same
    /// file could both read the pre-mutation doc and each overwrite the other's zoom when they
    /// save back.
    #[test]
    fn concurrent_apply_edit_op_calls_never_lose_a_write() {
        let folder = tmp_folder("concurrent");
        let p = paths(&folder);
        // Seed a non-degenerate doc directly so the write race is what's under test, not
        // `load_or_seed`'s build_default path (which needs a real event log).
        let mut seed = EditDoc::default();
        seed.clip_ms = 60_000;
        seed.trim.out_ms = 60_000;
        seed.save(&p.edit()).unwrap();

        const N: usize = 12;
        let handles: Vec<_> = (0..N).map(|i| {
            let folder = folder.clone();
            std::thread::spawn(move || {
                apply_edit_op(folder, EditOp::AddZoom { at_ms: (i as u32) * 1000, dur_ms: 500 }).unwrap();
            })
        }).collect();
        for h in handles { h.join().unwrap(); }

        let doc = EditDoc::load(&p.edit()).unwrap();
        assert_eq!(doc.zooms.len(), N, "every concurrent add must survive, got ids {:?}",
            doc.zooms.iter().map(|z| &z.id).collect::<Vec<_>>());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// H3 round 2: the review that caught round 1's gap - `seed::load_or_seed` is called
    /// UNLOCKED from ~8 places outside `edit::commands` (`ai::commands`,
    /// `export::preview::session`, `export::render::*`, `export::cursor::cursorpreview`,
    /// `export::preview::{preprocess,thumbs}`). Mixing direct `load_or_seed` calls (standing in
    /// for those other callers) with `apply_edit_op` calls on a doc that ALSO needs migrating (so
    /// every `load_or_seed` call in this test takes its OWN write path, not just a read) must
    /// never lose a concurrently-applied zoom - proving the lock `seed_lock`/`edit::lock` now
    /// take covers those callers too, not just the three commands in this file.
    #[test]
    fn concurrent_load_or_seed_calls_never_clobber_a_concurrent_apply_edit_op() {
        let folder = tmp_folder("mixed-concurrent");
        let p = paths(&folder);
        // A v1 doc missing clip_ms, so EVERY `load_or_seed` call below takes the slow
        // (migrate + write) path, not just a cheap read.
        let seed = EditDoc { version: 1, trim: crate::edit::model::Trim { in_ms: 0, out_ms: 60_000 }, ..Default::default() };
        seed.save(&p.edit()).unwrap();

        const N: usize = 8;
        let handles: Vec<_> = (0..N * 2).map(|i| {
            let folder = folder.clone();
            std::thread::spawn(move || {
                if i % 2 == 0 {
                    let _ = apply_edit_op(folder, EditOp::AddZoom { at_ms: (i as u32) * 100, dur_ms: 50 });
                } else {
                    let _ = crate::edit::seed::load_or_seed(&paths(&folder));
                }
            })
        }).collect();
        for h in handles { h.join().unwrap(); }

        let doc = EditDoc::load(&p.edit()).unwrap();
        assert_eq!(doc.version, crate::edit::model::DOC_VERSION, "migration must have completed");
        assert_eq!(doc.zooms.len(), N, "every apply_edit_op-added zoom must survive the mixed race, got ids {:?}",
            doc.zooms.iter().map(|z| &z.id).collect::<Vec<_>>());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
