// Camera-move op tests (AddCameraMove/UpdateCameraMove/RemoveCameraMove), split out of
// api_tests.rs so that file stays under the size limit. Mirrors the *LayoutSeg op tests.
use super::*;

#[test]
fn add_camera_move_appends_with_auto_id_and_default_easing() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 500, x: 0.5, y: 0.5, size: 0.3 });
    let m = &doc.camera_moves[0];
    assert_eq!((m.t_ms, m.x, m.y, m.size, m.easing.as_str()), (500, 0.5, 0.5, 0.3, "smooth"));
    assert!(m.id.starts_with('k'));
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 200, x: 0.1, y: 0.1, size: 0.1 });
    assert_ne!(doc.camera_moves[0].id, doc.camera_moves[1].id);
}

#[test]
fn add_camera_move_clamps_t_ms_to_trim_duration() {
    // Same "must not overflow past the clip" guard as add_zoom - a keyframe placed near the
    // end of a short clip shouldn't land past the clip's own duration.
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 5000, x: 0.5, y: 0.5, size: 0.3 });
    assert_eq!(doc.camera_moves[0].t_ms, 1000);
}

#[test]
fn add_camera_move_clamps_x_y_size_to_unit_range() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 0, x: -0.5, y: 1.5, size: 99.0 });
    let m = &doc.camera_moves[0];
    assert_eq!((m.x, m.y, m.size), (0.0, 1.0, 1.0));
}

#[test]
fn add_camera_move_keeps_vec_sorted_by_t_ms() {
    let mut doc = empty(); doc.trim.out_ms = 10000;
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 800, x: 0.0, y: 0.0, size: 0.2 });
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 200, x: 0.0, y: 0.0, size: 0.2 });
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 500, x: 0.0, y: 0.0, size: 0.2 });
    let ts: Vec<u32> = doc.camera_moves.iter().map(|m| m.t_ms).collect();
    assert_eq!(ts, vec![200, 500, 800]);
}

#[test]
fn update_camera_move_patches_only_supplied_fields() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 0, x: 0.2, y: 0.2, size: 0.2 });
    let id = doc.camera_moves[0].id.clone();
    apply(&mut doc, EditOp::UpdateCameraMove { id: id.clone(), t_ms: None, x: Some(0.9),
        y: None, size: None, easing: None });
    let m = &doc.camera_moves[0];
    assert_eq!((m.x, m.y, m.size), (0.9, 0.2, 0.2));
    apply(&mut doc, EditOp::UpdateCameraMove { id, t_ms: None, x: None, y: Some(0.7),
        size: Some(0.4), easing: Some("spring".into()) });
    let m = &doc.camera_moves[0];
    assert_eq!((m.y, m.size, m.easing.as_str()), (0.7, 0.4, "spring"));
}

#[test]
fn update_camera_move_clamps_and_ignores_unknown_id() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 0, x: 0.0, y: 0.0, size: 0.0 });
    apply(&mut doc, EditOp::UpdateCameraMove { id: "k999".into(), t_ms: Some(1), x: None, y: None, size: None, easing: None });
    assert_eq!(doc.camera_moves[0].t_ms, 0); // unknown id -> no mutation
    let id = doc.camera_moves[0].id.clone();
    apply(&mut doc, EditOp::UpdateCameraMove { id: id.clone(), t_ms: Some(9999), x: Some(-1.0), y: Some(2.0), size: None, easing: None });
    let m = &doc.camera_moves[0];
    assert_eq!((m.t_ms, m.x, m.y), (1000, 0.0, 1.0));
}

#[test]
fn update_camera_move_re_sorts_when_t_ms_changes() {
    let mut doc = empty(); doc.trim.out_ms = 10000;
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 100, x: 0.0, y: 0.0, size: 0.2 });
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 900, x: 0.0, y: 0.0, size: 0.2 });
    let first_id = doc.camera_moves[0].id.clone();
    apply(&mut doc, EditOp::UpdateCameraMove { id: first_id.clone(), t_ms: Some(9999),
        x: None, y: None, size: None, easing: None });
    // The retimed keyframe now sorts last.
    assert_eq!(doc.camera_moves.last().unwrap().id, first_id);
    assert_eq!(doc.camera_moves.iter().map(|m| m.t_ms).collect::<Vec<_>>(), vec![900, 9999]);
}

#[test]
fn remove_camera_move_drops_by_id() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 0, x: 0.0, y: 0.0, size: 0.2 });
    apply(&mut doc, EditOp::AddCameraMove { t_ms: 100, x: 0.0, y: 0.0, size: 0.2 });
    let id = doc.camera_moves[0].id.clone();
    let before = doc.camera_moves.len();
    apply(&mut doc, EditOp::RemoveCameraMove { id });
    assert_eq!(doc.camera_moves.len(), before - 1);
}
