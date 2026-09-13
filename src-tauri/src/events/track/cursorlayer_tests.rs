use super::*;

/// A throwaway project folder, unique per test name and process.
fn tmp(tag: &str) -> ProjectPaths {
    let dir = std::env::temp_dir().join(format!("tcursor-layer-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ProjectPaths { folder: dir }
}

fn cursor(w: u32, h: u32, hx: u32, hy: u32) -> CapturedCursor {
    CapturedCursor { w, h, hx, hy, rgba: vec![255u8; (w * h * 4) as usize] }
}

#[test]
fn save_then_load_round_trips_the_entries_and_the_track() {
    let paths = tmp("roundtrip");
    let mut b = CursorLayerBuilder::default();
    let arrow = b.add(cursor(32, 32, 5, 2));
    let ibeam = b.add(cursor(16, 24, 8, 12));
    b.mark(0, arrow);
    b.mark(500, ibeam);
    b.save(&paths).unwrap();

    let back = CursorLayer::load(&paths).unwrap();
    assert_eq!(back, b.layer());
    assert_eq!(back.cursors[1], CursorEntry { id: 1, w: 16, h: 24, hx: 8, hy: 12, file: "1.png".into() });
    // One real PNG per entry, at the path the entry names.
    for e in &back.cursors {
        let bytes = std::fs::read(paths.cursor_dir().join(&e.file)).unwrap();
        assert_eq!(&bytes[1..4], b"PNG", "{} is not a PNG", e.file);
    }
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn id_at_finds_the_sample_in_force_and_nothing_before_the_first() {
    let layer = CursorLayer { cursors: vec![], track: vec![(100, 0), (400, 1), (900, 0)] };
    assert_eq!(layer.id_at(0), None, "before the first sample nothing was captured yet");
    assert_eq!(layer.id_at(99), None);
    assert_eq!(layer.id_at(100), Some(0), "exactly on a boundary the new sample applies");
    assert_eq!(layer.id_at(399), Some(0));
    assert_eq!(layer.id_at(400), Some(1));
    assert_eq!(layer.id_at(u32::MAX), Some(0), "after the last sample it stays in force");
    assert_eq!(CursorLayer::default().id_at(0), None, "an empty track has no cursor");
}

#[test]
fn a_recording_without_a_layer_reports_no_layer_instead_of_failing() {
    let paths = tmp("absent");
    assert!(!CursorLayer::exists(&paths));
    assert!(CursorLayer::load(&paths).is_none());
    // Corrupt JSON is the same answer - never an error, never a panic.
    std::fs::create_dir_all(paths.cursor_dir()).unwrap();
    std::fs::write(paths.cursor_layer(), b"{not json").unwrap();
    assert!(CursorLayer::exists(&paths), "the file is there, so the video was captured clean");
    assert!(CursorLayer::load(&paths).is_none());
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn an_empty_layer_is_still_written_so_the_clean_video_is_recorded() {
    // Every GetIconInfo failing must NOT look like a pre-layer recording - that would make the
    // editor believe the OS cursor is baked into the pixels and draw nothing forever.
    let paths = tmp("empty");
    CursorLayerBuilder::default().save(&paths).unwrap();
    assert!(CursorLayer::exists(&paths));
    assert_eq!(CursorLayer::load(&paths).unwrap(), CursorLayer::default());
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[test]
fn the_builder_stops_accepting_bitmaps_at_the_cap() {
    let mut b = CursorLayerBuilder::default();
    for i in 0..MAX_CURSORS {
        assert!(!b.is_full(), "still room at {i}");
        b.add(cursor(1, 1, 0, 0));
    }
    assert!(b.is_full());
    assert_eq!(b.layer().cursors.len(), MAX_CURSORS);
}
