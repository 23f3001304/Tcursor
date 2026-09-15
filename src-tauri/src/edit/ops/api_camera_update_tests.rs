use super::*;

#[test]
fn update_camera_move_patches_only_supplied_fields() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 0,
            x: 0.2,
            y: 0.2,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    let id = doc.camera_moves[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id: id.clone(),
            t_ms: None,
            x: Some(0.9),
            y: None,
            size: None,
            easing: None,
            shape: None,
            roundness: None,
        },
    );
    let m = &doc.camera_moves[0];
    assert_eq!((m.x, m.y, m.size), (0.9, 0.2, 0.2));
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id,
            t_ms: None,
            x: None,
            y: Some(0.7),
            size: Some(0.4),
            easing: Some("spring".into()),
            shape: None,
            roundness: None,
        },
    );
    let m = &doc.camera_moves[0];
    assert_eq!((m.y, m.size, m.easing.as_str()), (0.7, 0.4, "spring"));
}

#[test]
fn update_camera_move_clamps_and_ignores_unknown_id() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 0,
            x: 0.0,
            y: 0.0,
            size: 0.0,
            shape: None,
            roundness: None,
        },
    );
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id: "k999".into(),
            t_ms: Some(1),
            x: None,
            y: None,
            size: None,
            easing: None,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves[0].t_ms, 0);
    let id = doc.camera_moves[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id: id.clone(),
            t_ms: Some(9999),
            x: Some(-1.0),
            y: Some(2.0),
            size: None,
            easing: None,
            shape: None,
            roundness: None,
        },
    );
    let m = &doc.camera_moves[0];
    assert_eq!((m.t_ms, m.x, m.y), (1000, 0.0, 1.0));
}

#[test]
fn update_camera_move_re_sorts_when_t_ms_changes() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
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
    apply(
        &mut doc,
        EditOp::AddCameraMove {
            t_ms: 900,
            x: 0.0,
            y: 0.0,
            size: 0.2,
            shape: None,
            roundness: None,
        },
    );
    let first_id = doc.camera_moves[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateCameraMove {
            id: first_id.clone(),
            t_ms: Some(9999),
            x: None,
            y: None,
            size: None,
            easing: None,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(doc.camera_moves.last().unwrap().id, first_id);
    assert_eq!(
        doc.camera_moves.iter().map(|m| m.t_ms).collect::<Vec<_>>(),
        vec![900, 9999]
    );
}
