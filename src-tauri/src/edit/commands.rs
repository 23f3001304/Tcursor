use crate::edit::model::EditDoc;
use crate::edit::ops::api::EditOp;
use crate::session::paths::ProjectPaths;

fn paths(folder: &str) -> ProjectPaths {
    ProjectPaths {
        folder: std::path::PathBuf::from(folder),
    }
}

#[tauri::command]
pub async fn get_edit(folder: String) -> Result<EditDoc, String> {
    tauri::async_runtime::spawn_blocking(move || crate::edit::seed::load_or_seed(&paths(&folder)))
        .await
        .map_err(|e| e.to_string())
}

// One lock acquisition covers BOTH `load_or_seed`'s possible seed/migrate/lift write AND this
// op's own apply+save (H3, bug-sweep-2 Task 7 round 2) - never two separate acquisitions.
// `edit::lock::doc_lock` is a plain `std::sync::Mutex`, NOT reentrant, so calling the
// self-locking `seed::load_or_seed` from inside an already-held lock would deadlock; this
// precomputes the same ffprobe-backed inputs `load_or_seed` would (unlocked, via
// `seed::derive_seed_inputs`), then does everything else - the seed/migrate/lift decision
// AND this command's own op+save - under one lock, via `seed::load_or_seed_locked`.
#[tauri::command]
pub fn apply_edit_op(folder: String, op: EditOp) -> Result<EditDoc, String> {
    let p = paths(&folder);
    let unlocked = EditDoc::load(&p.edit());
    let (precomputed_default, shift, true_dur) =
        crate::edit::seed::derive_seed_inputs(&p, &unlocked);
    let lock = crate::edit::lock::doc_lock(&p);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut doc = crate::edit::seed::load_or_seed_locked(&p, precomputed_default, shift, true_dur);
    crate::edit::ops::api::apply(&mut doc, op.clone());
    if let EditOp::UpdateZoom {
        id,
        start_ms,
        smart_typing,
        ..
    } = &op
    {
        if start_ms.is_some() || *smart_typing == Some(true) {
            crate::edit::ops::smart_zoom::refit(&mut doc, id, &p);
        }
    }
    doc.save(&p.edit()).map_err(|e| e.to_string())?;
    Ok(doc)
}

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
        let dir =
            std::env::temp_dir().join(format!("tcursor-editcmd-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn concurrent_apply_edit_op_calls_never_lose_a_write() {
        let folder = tmp_folder("concurrent");
        let p = paths(&folder);
        let mut seed = EditDoc::default();
        seed.clip_ms = 60_000;
        seed.trim.out_ms = 60_000;
        seed.save(&p.edit()).unwrap();

        const N: usize = 12;
        let handles: Vec<_> = (0..N)
            .map(|i| {
                let folder = folder.clone();
                std::thread::spawn(move || {
                    apply_edit_op(
                        folder,
                        EditOp::AddZoom {
                            at_ms: (i as u32) * 1000,
                            dur_ms: 500,
                        },
                    )
                    .unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let doc = EditDoc::load(&p.edit()).unwrap();
        assert_eq!(
            doc.zooms.len(),
            N,
            "every concurrent add must survive, got ids {:?}",
            doc.zooms.iter().map(|z| &z.id).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn concurrent_load_or_seed_calls_never_clobber_a_concurrent_apply_edit_op() {
        let folder = tmp_folder("mixed-concurrent");
        let p = paths(&folder);
        let seed = EditDoc {
            version: 1,
            trim: crate::edit::model::Trim {
                in_ms: 0,
                out_ms: 60_000,
            },
            ..Default::default()
        };
        seed.save(&p.edit()).unwrap();

        const N: usize = 8;
        let handles: Vec<_> = (0..N * 2)
            .map(|i| {
                let folder = folder.clone();
                std::thread::spawn(move || {
                    if i % 2 == 0 {
                        let _ = apply_edit_op(
                            folder,
                            EditOp::AddZoom {
                                at_ms: (i as u32) * 100,
                                dur_ms: 50,
                            },
                        );
                    } else {
                        let _ = crate::edit::seed::load_or_seed(&paths(&folder));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let doc = EditDoc::load(&p.edit()).unwrap();
        assert_eq!(
            doc.version,
            crate::edit::model::DOC_VERSION,
            "migration must have completed"
        );
        assert_eq!(
            doc.zooms.len(),
            N,
            "every apply_edit_op-added zoom must survive the mixed race, got ids {:?}",
            doc.zooms.iter().map(|z| &z.id).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&folder);
    }
}
