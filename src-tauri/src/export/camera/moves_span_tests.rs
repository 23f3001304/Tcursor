// Task 27 span semantics for export::camera::moves: keyframes own only
// `[first - KF_BLEND_MS, last + KF_BLEND_MS]`, easing to and from the LIVE layout-resolved pose
// at each edge. Split from moves_tests.rs (which keeps the in-span interpolation cases) so both
// files stay under the size limit.
use super::*;
use crate::edit::model::CameraMove;

fn kf(t_ms: u32, x: f32, y: f32, size: f32, easing: &str) -> CameraMove {
    CameraMove { id: "k".into(), t_ms, x, y, size, easing: easing.into() }
}

const LIVE: CamPose = CamPose { x: 0.85, y: 0.15, size: 0.18 };

/// Two keyframes at 2000/4000 - the span is [1650, 4350].
fn two_kf_track() -> CameraMoveTrack {
    CameraMoveTrack::from_doc(&[kf(2000, 0.20, 0.70, 0.10, "linear"), kf(4000, 0.60, 0.30, 0.40, "linear")])
}

#[test]
fn span_is_the_keyframe_range_padded_by_one_blend_each_side() {
    assert_eq!(two_kf_track().span(), Some((2000 - KF_BLEND_MS, 4000 + KF_BLEND_MS)));
    // Clamped at 0 rather than wrapping when the first keyframe is inside one blend of the start.
    assert_eq!(CameraMoveTrack::from_doc(&[kf(100, 0.5, 0.5, 0.2, "linear")]).span(), Some((0, 100 + KF_BLEND_MS)));
}

#[test]
fn outside_the_span_the_layout_owns_the_panel() {
    let track = two_kf_track();
    assert_eq!(track.sample(1000, Some(LIVE)), None, "1000 < first(2000) - 350");
    assert_eq!(track.sample(5000, Some(LIVE)), None, "5000 > last(4000) + 350");
    // Exact boundaries are INSIDE (that is where the blends begin/end).
    assert_eq!(track.sample(1649, Some(LIVE)), None);
    assert!(track.sample(1650, Some(LIVE)).is_some());
    assert!(track.sample(4350, Some(LIVE)).is_some());
    assert_eq!(track.sample(4351, Some(LIVE)), None);
}

#[test]
fn entry_blend_is_continuous_at_both_ends() {
    let track = two_kf_track();
    let want_kf = CamPose { x: 0.20, y: 0.70, size: 0.10 };

    let at_start = track.sample(2000 - KF_BLEND_MS, Some(LIVE)).unwrap();
    assert!((at_start.x - LIVE.x).abs() < 1e-4, "x={}", at_start.x);
    assert!((at_start.y - LIVE.y).abs() < 1e-4, "y={}", at_start.y);
    assert!((at_start.size - LIVE.size).abs() < 1e-4, "size={}", at_start.size);

    let at_first = track.sample(2000, Some(LIVE)).unwrap();
    assert!((at_first.x - want_kf.x).abs() < 1e-4, "x={}", at_first.x);
    assert!((at_first.y - want_kf.y).abs() < 1e-4, "y={}", at_first.y);
    assert!((at_first.size - want_kf.size).abs() < 1e-4, "size={}", at_first.size);

    let mid = track.sample(2000 - KF_BLEND_MS / 2, Some(LIVE)).unwrap();
    assert!(mid.x < LIVE.x && mid.x > want_kf.x, "x={}", mid.x);
    assert!(mid.y > LIVE.y && mid.y < want_kf.y, "y={}", mid.y);
    assert!(mid.size < LIVE.size && mid.size > want_kf.size, "size={}", mid.size);
}

#[test]
fn exit_blend_tracks_a_live_pose_that_is_still_moving() {
    // A layout transition running THROUGH the exit window: live(t) sweeps linearly from
    // (0.2,0.2,0.1) at t=last to (0.8,0.6,0.3) at t=last+BLEND. Re-evaluating live per frame
    // means the blend must land on live(last+BLEND) - the moving target - not the stale
    // live(last) it started from.
    let track = two_kf_track();
    let live_at = |t: u32| {
        let u = (t - 4000) as f32 / KF_BLEND_MS as f32;
        CamPose { x: 0.2 + 0.6 * u, y: 0.2 + 0.4 * u, size: 0.1 + 0.2 * u }
    };
    let end_t = 4000 + KF_BLEND_MS;
    let got = track.sample(end_t, Some(live_at(end_t))).unwrap();
    let want = live_at(end_t); // (0.8, 0.6, 0.3)
    assert!((got.x - want.x).abs() < 1e-4, "x={} want {}", got.x, want.x);
    assert!((got.y - want.y).abs() < 1e-4, "y={} want {}", got.y, want.y);
    assert!((got.size - want.size).abs() < 1e-4, "size={} want {}", got.size, want.size);
    // ...and it is NOT the stale live(last) the exit started from.
    assert!((got.x - live_at(4000).x).abs() > 0.5, "must not hold the stale start pose");
    // At t == last the exit hasn't begun: the keyframe pose still wins exactly.
    let at_last = track.sample(4000, Some(live_at(4000))).unwrap();
    assert!((at_last.x - 0.60).abs() < 1e-4, "x={}", at_last.x);
}

#[test]
fn single_keyframe_is_a_bump_not_a_whole_clip_override() {
    let track = CameraMoveTrack::from_doc(&[kf(3000, 0.25, 0.75, 0.30, "linear")]);
    assert_eq!(track.span(), Some((2650, 3350)));
    assert_eq!(track.sample(2649, Some(LIVE)), None, "before the ease-in window: layout owns it");
    assert_eq!(track.sample(3351, Some(LIVE)), None, "after the ease-out window: layout owns it");
    let at_kf = track.sample(3000, Some(LIVE)).unwrap();
    assert_eq!(at_kf, CamPose { x: 0.25, y: 0.75, size: 0.30 });
    // The two window edges sit exactly on the live pose (continuous in and out).
    for t in [2650u32, 3350u32] {
        let p = track.sample(t, Some(LIVE)).unwrap();
        assert!((p.x - LIVE.x).abs() < 1e-4 && (p.size - LIVE.size).abs() < 1e-4, "t={} p={:?}", t, p);
    }
}

#[test]
fn without_a_live_pose_the_blends_snap_to_the_nearest_end_keyframe() {
    // The span rule never depends on `live`; only the two blends do.
    let track = two_kf_track();
    assert_eq!(track.sample(1000, None), None);
    assert_eq!(track.sample(5000, None), None);
    assert_eq!(track.sample(1700, None), Some(CamPose { x: 0.20, y: 0.70, size: 0.10 }));
    assert_eq!(track.sample(4300, None), Some(CamPose { x: 0.60, y: 0.30, size: 0.40 }));
}
