// Tests for export::camera::moves, split into their own file so moves.rs stays under the size
// limit. This file covers the IN-SPAN interpolation (unchanged by Task 27); the span/blend
// semantics live in moves_span_tests.rs.
use super::*;
use crate::edit::model::CameraMove;

fn kf(t_ms: u32, x: f32, y: f32, size: f32, easing: &str) -> CameraMove {
    CameraMove { id: "k".into(), t_ms, x, y, size, easing: easing.into() }
}

#[test]
fn empty_track_samples_to_none() {
    let track = CameraMoveTrack::from_doc(&[]);
    assert_eq!(track.span(), None);
    assert_eq!(track.sample(0, None), None);
    assert_eq!(track.sample(5000, None), None);
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
    // Sorted-stable duplicate t_ms: at exactly t=1000 the straddling pair is (the LATER of the
    // two coincident kfs, the 2000 one) with f=0, so one of the two coincident poses comes back.
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

// Task 27 regression guard: the MID-SPAN math is untouched by the span/blend rewrite. Keyframes
// at 2000/4000 sampled at 3000 must still produce the pre-change interpolation values, computed
// here the way the old code did (`a + (b-a) * ease(b.easing, (t-a)/(b-a))`), independently of
// `sample`'s own implementation - and it must not depend on `live` at all.
#[test]
fn mid_span_interpolation_is_unchanged_by_the_span_rewrite() {
    let moves = vec![kf(2000, 0.10, 0.80, 0.12, "smooth"), kf(4000, 0.70, 0.20, 0.34, "smooth")];
    let track = CameraMoveTrack::from_doc(&moves);
    let f = ease(Easing::Smooth, (3000.0 - 2000.0) / (4000.0 - 2000.0)); // == smoothstep(0.5) == 0.5
    let want = CamPose { x: 0.10 + (0.70 - 0.10) * f, y: 0.80 + (0.20 - 0.80) * f, size: 0.12 + (0.34 - 0.12) * f };
    let live = CamPose { x: 0.95, y: 0.05, size: 0.9 }; // must have no influence mid-span
    for got in [track.sample(3000, None).unwrap(), track.sample(3000, Some(live)).unwrap()] {
        assert!((got.x - want.x).abs() < 1e-4, "x={} want {}", got.x, want.x);
        assert!((got.y - want.y).abs() < 1e-4, "y={} want {}", got.y, want.y);
        assert!((got.size - want.size).abs() < 1e-4, "size={} want {}", got.size, want.size);
    }
    assert!((want.x - 0.40).abs() < 1e-6, "pinned expected value drifted: {}", want.x);
}
