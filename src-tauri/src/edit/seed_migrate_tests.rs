use super::*;

#[test]
fn v1_docs_migrate_their_regions_once() {
    let paths = fixture("migrate_v1", vec![]);
    EditDoc {
        version: 1,
        trim: Trim {
            in_ms: 0,
            out_ms: 5000,
        },
        effects: vec![effect("e0", 2000, 3000), effect("e1", 100, 400)],
        layout: vec![seg("l0", 0, 2000), seg("l1", 2000, 5000)],
        ..Default::default()
    }
    .save(&paths.edit())
    .unwrap();

    let doc = load_or_seed(&paths);
    assert_eq!(doc.version, DOC_VERSION);
    assert_eq!(
        spans(&doc),
        (vec![(1200, 2200), (0, 0)], vec![(0, 1200), (1200, 4200)])
    );
    assert_eq!(
        doc.clip_ms, 5000,
        "migrate backfills clip_ms from the true recording length too"
    );

    let again = load_or_seed(&paths);
    assert_eq!(spans(&again), spans(&doc), "migration must be idempotent");
    assert_eq!(again.version, DOC_VERSION);
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn a_v1_doc_without_a_timeline_becomes_v2_unshifted() {
    let paths = ProjectPaths {
        folder: std::env::temp_dir().join("tcursor_seed_migrate_no_timeline"),
    };
    let _ = std::fs::remove_dir_all(&paths.folder);
    paths.ensure().unwrap();
    EditDoc {
        version: 1,
        trim: Trim {
            in_ms: 0,
            out_ms: 5000,
        },
        effects: vec![effect("e0", 2000, 3000)],
        layout: vec![seg("l0", 0, 5000)],
        ..Default::default()
    }
    .save(&paths.edit())
    .unwrap();
    let doc = load_or_seed(&paths);
    assert_eq!(doc.version, DOC_VERSION);
    assert_eq!(spans(&doc), (vec![(2000, 3000)], vec![(0, 5000)]));
    assert_eq!(
        doc.clip_ms, 0,
        "clip_ms stays unknown too - same reason as the regions"
    );
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn clip_ms_backfills_on_an_already_current_version_doc_too() {
    let paths = fixture("clip_ms_backfill_v2", vec![]);
    EditDoc {
        version: DOC_VERSION,
        trim: Trim {
            in_ms: 0,
            out_ms: 5000,
        },
        ..Default::default()
    }
    .save(&paths.edit())
    .unwrap();
    let doc = load_or_seed(&paths);
    assert_eq!(doc.clip_ms, 5000);
    let _ = std::fs::remove_dir_all(&paths.folder);
}
