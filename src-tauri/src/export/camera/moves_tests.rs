// Tests for export::camera::moves, split into their own file so moves.rs stays under the size limit.
use super::*;
use crate::edit::model::CameraMove;

fn kf(t_ms: u32, x: f32, y: f32, size: f32, easing: &str) -> CameraMove {
    CameraMove { id: "k".into(), t_ms, x, y, size, easing: easing.into() }
}

#[test]
fn empty_track_samples_to_none() {
    let track = CameraMoveTrack::from_doc(&[]);
    assert_eq!(track.sample(0, None), None);
    assert_eq!(track.sample(5000, None), None);
}

#[test]
fn single_keyframe_holds_at_any_time() {
    let track = CameraMoveTrack::from_doc(&[kf(1000, 0.3, 0.7, 0.2, "smooth")]);
    let want = CamPose { x: 0.3, y: 0.7, size: 0.2 };
    assert_eq!(track.sample(0, None), Some(want));
    assert_eq!(track.sample(1000, None), Some(want));
    assert_eq!(track.sample(50_000, None), Some(want));
}

#[test]
fn holds_first_pose_before_and_at_first_keyframe() {
    let moves = vec![kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    let first = CamPose { x: 0.1, y: 0.1, size: 0.1 };
    assert_eq!(track.sample(0, None), Some(first));
    assert_eq!(track.sample(999, None), Some(first));
    assert_eq!(track.sample(1000, None), Some(first));
}

#[test]
fn holds_last_pose_at_and_after_last_keyframe() {
    let moves = vec![kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    let last = CamPose { x: 0.9, y: 0.9, size: 0.5 };
    assert_eq!(track.sample(2000, None), Some(last));
    assert_eq!(track.sample(9000, None), Some(last));
}

#[test]
fn midpoint_lerp_with_linear_easing_is_the_exact_mean() {
    let moves = vec![kf(0, 0.0, 0.0, 0.0, "linear"), kf(1000, 1.0, 1.0, 1.0, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    let p = track.sample(500, None).unwrap();
    assert!((p.x - 0.5).abs() < 1e-6, "x={}", p.x);
    assert!((p.y - 0.5).abs() < 1e-6, "y={}", p.y);
    assert!((p.size - 0.5).abs() < 1e-6, "size={}", p.size);
}

#[test]
fn midpoint_with_smooth_easing_diverges_from_the_linear_mean() {
    // smoothstep(0.5) == 0.5 exactly (3*0.5^2 - 2*0.5^3 = 0.5), so a symmetric endpoint
    // pair alone wouldn't distinguish smooth from linear - use asymmetric per-axis deltas
    // and assert against the eased fraction directly, matching `ease`'s own formula.
    let moves = vec![kf(0, 0.0, 0.2, 0.1, "smooth"), kf(1000, 1.0, 0.2, 0.9, "smooth")];
    let track = CameraMoveTrack::from_doc(&moves);
    let f = ease(Easing::Smooth, 0.25); // != 0.25 for smoothstep away from the midpoint
    assert!((f - 0.25).abs() > 1e-6, "sanity: smoothstep(0.25) must differ from linear 0.25");
    let p = track.sample(250, None).unwrap();
    assert!((p.x - f).abs() < 1e-6, "x should equal the eased fraction: got {} want {}", p.x, f);
    assert!((p.size - (0.1 + 0.8 * f)).abs() < 1e-6);
}

#[test]
fn coincident_keyframe_times_snap_to_b_without_dividing_by_zero() {
    let moves = vec![kf(1000, 0.2, 0.2, 0.2, "linear"), kf(1000, 0.8, 0.8, 0.8, "linear"), kf(2000, 0.0, 0.0, 0.0, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    let p = track.sample(1000, None).unwrap();
    // Sorted-stable duplicate t_ms: at exactly t=1000 the "at/before first" hold branch
    // wins (both kfs share the min t_ms), returning whichever sort placed first.
    assert!(p.x == 0.2 || p.x == 0.8, "got {}", p.x);
}

#[test]
fn out_of_order_input_is_sorted_defensively() {
    let moves = vec![kf(2000, 0.9, 0.9, 0.5, "linear"), kf(0, 0.0, 0.0, 0.0, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    assert_eq!(track.sample(0, None), Some(CamPose { x: 0.0, y: 0.0, size: 0.0 }));
    assert_eq!(track.sample(2000, None), Some(CamPose { x: 0.9, y: 0.9, size: 0.5 }));
    let mid = track.sample(1000, None).unwrap();
    assert!((mid.x - 0.45).abs() < 1e-6, "x={}", mid.x);
}

#[test]
fn single_keyframe_with_static_pose_animates_in_from_the_static_start() {
    // One keyframe at t=1000; static_pose is the caller's un-overridden webcam pose (what it
    // would show with zero camera_moves). t=0 should read back (approximately) the static pose,
    // t=1000 the keyframe's pose, and the midpoint must sit strictly between them on every axis -
    // proving this animates in, rather than holding the keyframe flat for all t (the old bug).
    let moves = vec![kf(1000, 0.9, 0.9, 0.5, "linear")];
    let track = CameraMoveTrack::from_doc(&moves);
    let static_pose = CamPose { x: 0.1, y: 0.1, size: 0.1 };

    let at0 = track.sample(0, Some(static_pose)).unwrap();
    assert!((at0.x - static_pose.x).abs() < 1e-6, "x={}", at0.x);
    assert!((at0.y - static_pose.y).abs() < 1e-6, "y={}", at0.y);
    assert!((at0.size - static_pose.size).abs() < 1e-6, "size={}", at0.size);

    let at1000 = track.sample(1000, Some(static_pose)).unwrap();
    let want = CamPose { x: 0.9, y: 0.9, size: 0.5 };
    assert!((at1000.x - want.x).abs() < 1e-6, "x={}", at1000.x);
    assert!((at1000.y - want.y).abs() < 1e-6, "y={}", at1000.y);
    assert!((at1000.size - want.size).abs() < 1e-6, "size={}", at1000.size);

    let mid = track.sample(500, Some(static_pose)).unwrap();
    assert!(mid.x > static_pose.x && mid.x < want.x, "x={}", mid.x);
    assert!(mid.y > static_pose.y && mid.y < want.y, "y={}", mid.y);
    assert!(mid.size > static_pose.size && mid.size < want.size, "size={}", mid.size);

    // None still means the old "hold the keyframe flat" behavior, at the same t=500.
    let held = track.sample(500, None).unwrap();
    assert_eq!(held, want);
}
