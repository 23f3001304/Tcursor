use super::*;

/// The five instants the TS mirror (`src/editor/stage/cursorBusy.test.ts`) asserts the SAME
/// numbers at. Any change here has to be made there in the same edit, or export and preview
/// silently disagree about what the busy cursor is doing.
const INSTANTS: [u32; 5] = [0, 250, 500, 750, 1000];

fn spec(anim: BusyAnim, fps: f32) -> BusySpec {
    BusySpec { anim, fps, frames: 0 }
}

fn close(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 1e-3, "{what}: {a} != {b}");
}

#[test]
fn spin_turns_once_per_second_at_the_default_fps() {
    let expected = [0.0, 90.0, 180.0, 270.0, 0.0]; // 1000ms is a full turn, back to 0
    for (t, want) in INSTANTS.iter().zip(expected) {
        let p = busy_pose(&spec(BusyAnim::Spin, 24.0), *t);
        close(p.angle_deg, want, &format!("spin @{t}ms"));
        assert_eq!((p.frame, p.scale), (0, 1.0), "spin only rotates");
    }
}

#[test]
fn spin_fps_sets_the_cycle_length() {
    // A full turn takes 24/fps seconds, so half the fps is half the speed.
    close(busy_pose(&spec(BusyAnim::Spin, 12.0), 500).angle_deg, 90.0, "half-speed spin @500ms");
    close(busy_pose(&spec(BusyAnim::Spin, 48.0), 250).angle_deg, 180.0, "double-speed spin @250ms");
}

#[test]
fn flip_holds_then_turns_and_never_turns_back() {
    // Hold through 70% of the cycle, ease a half turn over the last 30%, and carry the completed
    // half-turns forward - at 1000ms it is a full 180 from where it started, not back at 0.
    let expected = [0.0, 0.0, 0.0, 12.0577, 180.0];
    for (t, want) in INSTANTS.iter().zip(expected) {
        let p = busy_pose(&spec(BusyAnim::Flip, 24.0), *t);
        close(p.angle_deg, want, &format!("flip @{t}ms"));
        assert_eq!((p.frame, p.scale), (0, 1.0), "flip only rotates");
    }
    // Monotonic across the turning window, and wrapping to 0 only after two half turns.
    assert!(busy_pose(&spec(BusyAnim::Flip, 24.0), 900).angle_deg > 12.06);
    close(busy_pose(&spec(BusyAnim::Flip, 24.0), 2000).angle_deg, 0.0, "two half turns is a full circle");
}

#[test]
fn pulse_breathes_to_its_peak_at_mid_cycle_and_back() {
    let expected = [1.0, 1.03, 1.06, 1.03, 1.0];
    for (t, want) in INSTANTS.iter().zip(expected) {
        let p = busy_pose(&spec(BusyAnim::Pulse, 24.0), *t);
        close(p.scale, want, &format!("pulse @{t}ms"));
        assert_eq!((p.frame, p.angle_deg), (0, 0.0), "pulse only scales");
    }
}

#[test]
fn explicit_frames_win_over_the_synthesised_animation() {
    // A pack that ships busy_00..busy_03 IS the animation: pick a frame, never transform.
    let s = BusySpec { anim: BusyAnim::Spin, fps: 8.0, frames: 4 };
    for (t, want) in [(0u32, 0u32), (125, 1), (250, 2), (375, 3), (500, 0), (1000, 0)] {
        let p = busy_pose(&s, t);
        assert_eq!(p.frame, want, "frame @{t}ms");
        assert!(p.is_identity(), "an explicit frame is drawn untransformed");
    }
}

#[test]
fn a_zero_fps_frame_pack_cannot_divide_by_zero() {
    let s = BusySpec { anim: BusyAnim::Spin, fps: 0.0, frames: 3 };
    assert_eq!(busy_pose(&s, 10_000).frame, 0);
}

#[test]
fn the_still_pose_takes_the_untransformed_fast_path() {
    assert!(BusyPose::still().is_identity());
    assert!(busy_pose(&spec(BusyAnim::Spin, 24.0), 0).is_identity(), "angle 0 is still the fast path");
    assert!(!busy_pose(&spec(BusyAnim::Spin, 24.0), 250).is_identity());
    assert!(!busy_pose(&spec(BusyAnim::Pulse, 24.0), 500).is_identity());
}

#[test]
fn anim_names_round_trip_through_the_pack_json_wire_form() {
    for (v, wire) in [(BusyAnim::Spin, "spin"), (BusyAnim::Flip, "flip"), (BusyAnim::Pulse, "pulse")] {
        assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{wire}\""));
        assert_eq!(serde_json::from_str::<BusyAnim>(&format!("\"{wire}\"")).unwrap(), v);
    }
    // `frames` is discovered by scanning the folder, so pack.json never carries it.
    let s: BusySpec = serde_json::from_str(r#"{"anim":"pulse","fps":24}"#).unwrap();
    assert_eq!(s, BusySpec { anim: BusyAnim::Pulse, fps: 24.0, frames: 0 });
}
