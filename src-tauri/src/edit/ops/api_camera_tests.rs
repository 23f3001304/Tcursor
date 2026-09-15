use super::*;

#[test]
fn add_camera_move_appends_with_auto_id_and_default_easing() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    let m = &doc.camera_moves[0];
    assert_eq!(
        (m.t_ms, m.x, m.y, m.size, m.easing.as_str()),
        (500, 0.5, 0.5, 0.3, "smooth")
    );
    assert!(m.id.starts_with('k'));
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 200,
            x: 0.1,
            y: 0.1,
            size: 0.1,
            shape: None,
            roundness: None,
        },
    );
    assert_ne!(doc.camera_moves[0].id, doc.camera_moves[1].id);
}

#[test]
fn add_camera_move_clamps_t_ms_to_trim_duration() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 5000,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves[0].t_ms, 1000);
}

#[test]
fn add_camera_move_clamps_x_y_size_to_unit_range() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 0,
            x: -0.5,
            y: 1.5,
            size: 99.0,
            shape: None,
            roundness: None,
        },
    );
    let m = &doc.camera_moves[0];
    assert_eq!((m.x, m.y, m.size), (0.0, 1.0, 1.0));
}

#[test]
fn add_camera_move_keeps_vec_sorted_by_t_ms() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 800,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 200,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    let ts: Vec<u32> = doc.camera_moves.iter().map(|m| m.t_ms).collect();
    assert_eq!(ts, vec![200, 500, 800]);
}

#[test]
fn remove_camera_move_drops_by_id() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 0,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 100,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    let id = doc.camera_moves[0].id.clone();
    let before = doc.camera_moves.len();
    apply(&mut doc, EditOp::RemoveCameraMove { id });
    assert_eq!(doc.camera_moves.len(), before - 1);
}

#[test]
fn add_camera_move_at_an_occupied_instant_updates_in_place() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.5,
            y: 0.5,
            size: 0.20,
            shape: None,
            roundness: None,
        },
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.6,
            y: 0.4,
            size: 0.35,
            shape: Some("circle".into()),
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves.len(), 1);
    let m = &doc.camera_moves[0];
    assert_eq!(
        (m.x, m.y, m.size, m.shape.as_str()),
        (0.6, 0.4, 0.35, "circle")
    );
    assert_eq!(
        m.id, "k0",
        "the keyframe keeps its identity (a selected inspector stays on it)"
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 530,
            x: 0.7,
            y: 0.3,
            size: 0.25,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(
        doc.camera_moves.len(),
        1,
        "inside CAM_KF_SNAP_MS is the same instant"
    );
    let m = &doc.camera_moves[0];
    assert_eq!(
        (m.t_ms, m.x, m.size, m.shape.as_str()),
        (500, 0.7, 0.25, "circle")
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 700,
            x: 0.1,
            y: 0.1,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves.len(), 2);
}

#[test]
fn camera_move_shapes_are_validated_and_default_to_layout() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 100,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(
        (
            doc.camera_moves[0].shape.as_str(),
            doc.camera_moves[0].roundness
        ),
        ("layout", DEFAULT_CAM_ROUNDNESS)
    );
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 200,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: Some("hexagon".into()),
            roundness: Some(4.0),
        },
    );
    assert_eq!(
        (
            doc.camera_moves[1].shape.as_str(),
            doc.camera_moves[1].roundness
        ),
        ("layout", 0.5)
    );
    let id = doc.camera_moves[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id: id.clone(),
            t_ms: None,
            x: None,
            y: None,
            size: None,
            easing: None,
            shape: Some("rounded".into()),
            roundness: Some(0.3),
        },
    );
    assert_eq!(
        (
            doc.camera_moves[0].shape.as_str(),
            doc.camera_moves[0].roundness
        ),
        ("rounded", 0.3)
    );
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id,
            t_ms: None,
            x: None,
            y: None,
            size: None,
            easing: None,
            shape: Some("blob".into()),
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves[0].shape, "layout");
}

#[path = "api_camera_update_tests.rs"]
mod camera_update_tests;
