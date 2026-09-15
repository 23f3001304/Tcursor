use super::*;
use crate::edit::model::{EditDoc, Trim};
use crate::edit::ops::api::{apply, EditOp};
use crate::events::model::{EventKind, EventLog, MouseEvent, ScreenInfo};
use crate::session::sync::SyncLog;

const VIDEO_START: u64 = 800;

fn fixture(name: &str) -> ProjectPaths {
    let paths = ProjectPaths {
        folder: std::env::temp_dir().join(format!("tcursor_seedlock_{name}")),
    };
    let _ = std::fs::remove_dir_all(&paths.folder);
    paths.ensure().unwrap();
    let log = EventLog {
        started_unix_ms: 0,
        screen: ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        },
        events: vec![MouseEvent {
            t: 0,
            kind: EventKind::Move,
            x: 10,
            y: 10,
            button: None,
        }],
    };
    log.save(&paths.events()).unwrap();
    SyncLog {
        frames: (0..=100).map(|k| VIDEO_START + k * 50).collect(),
        events_ms: 0,
        mic_ms: None,
        system_ms: None,
        ..Default::default()
    }
    .save(&paths.sync())
    .unwrap();
    paths
}

#[test]
fn needs_seed_write_is_false_for_a_fully_current_doc() {
    let doc = EditDoc {
        version: DOC_VERSION,
        clip_ms: 5000,
        ..Default::default()
    };
    assert!(!needs_seed_write(&doc));
}

#[test]
fn needs_seed_write_is_true_for_an_old_version() {
    let doc = EditDoc {
        version: 1,
        clip_ms: 5000,
        ..Default::default()
    };
    assert!(needs_seed_write(&doc));
}

#[test]
fn needs_seed_write_is_true_when_clip_ms_is_unknown() {
    let doc = EditDoc {
        version: DOC_VERSION,
        clip_ms: 0,
        ..Default::default()
    };
    assert!(needs_seed_write(&doc));
}

#[test]
fn needs_seed_write_is_true_when_the_spotlight_toggle_needs_lifting() {
    let mut doc = EditDoc {
        version: DOC_VERSION,
        clip_ms: 5000,
        ..Default::default()
    };
    doc.settings.clickfx.spotlight = true;
    assert!(needs_seed_write(&doc));
}

#[test]
fn derive_seed_inputs_skips_ffprobe_work_for_an_already_current_doc() {
    let paths = fixture("gating_current");
    let doc = EditDoc {
        version: DOC_VERSION,
        clip_ms: 5000,
        ..Default::default()
    };
    let (precomputed, shift, true_dur) = derive_seed_inputs(&paths, &Some(doc));
    assert!(
        precomputed.is_none(),
        "an existing doc must never precompute a fresh build_default"
    );
    assert_eq!(
        (shift, true_dur),
        (0, 0),
        "neither is needed - both stay at the gated-off default"
    );
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn locked_reread_wins_over_a_write_that_lands_between_derive_and_lock() {
    let paths = fixture("reread_wins");
    EditDoc {
        version: 1,
        trim: Trim {
            in_ms: 0,
            out_ms: 5000,
        },
        ..Default::default()
    }
    .save(&paths.edit())
    .unwrap();

    let unlocked = EditDoc::load(&paths.edit());
    let (precomputed_default, shift, true_dur) = derive_seed_inputs(&paths, &unlocked);

    let mut concurrent = EditDoc::load(&paths.edit()).unwrap();
    apply(
        &mut concurrent,
        EditOp::AddZoom {
            at_ms: 100,
            dur_ms: 200,
        },
    );
    concurrent.save(&paths.edit()).unwrap();

    let doc = load_or_seed_locked(&paths, precomputed_default, shift, true_dur);
    assert_eq!(
        doc.zooms.len(),
        1,
        "the concurrently-applied zoom must survive the seed/migrate write"
    );
    assert_eq!(
        doc.version, DOC_VERSION,
        "migration still completes on the fresh doc"
    );

    let reloaded = EditDoc::load(&paths.edit()).unwrap();
    assert_eq!(
        reloaded.zooms.len(),
        1,
        "and the save reflects it too, not just the in-memory return value"
    );
    let _ = std::fs::remove_dir_all(&paths.folder);
}
