use super::{audio_shift_ms, trim_frame_bounds, webcam_warning};

#[test]
fn webcam_warning_separates_zero_frames_from_a_partial_failure() {
    assert_eq!(
        webcam_warning(false, Some("boom"), 0),
        None,
        "no webcam file: never a warning"
    );
    assert_eq!(
        webcam_warning(true, None, 120),
        None,
        "a healthy decode says nothing"
    );
    let none = webcam_warning(true, None, 0).unwrap();
    assert!(
        none.contains("no frames") && none.contains("without the camera panel"),
        "{none}"
    );
    let early = webcam_warning(true, Some("moov atom not found"), 0).unwrap();
    assert!(
        early.contains("before any frame") && early.contains("moov atom not found"),
        "{early}"
    );
    let partial = webcam_warning(true, Some("read failed"), 431).unwrap();
    assert!(
        partial.contains("after 431 frames") && partial.contains("frozen"),
        "{partial}"
    );
    assert!(
        !partial.contains("without the camera panel"),
        "a frozen panel is not an absent one: {partial}"
    );
}

#[test]
fn trim_frame_bounds_converts_ms_to_inclusive_frame_indices() {
    assert_eq!(trim_frame_bounds(0, 10_000, 60), (0, 600));
    assert_eq!(trim_frame_bounds(2_000, 8_000, 60), (120, 480));
}

#[test]
fn trim_frame_bounds_never_collapses_to_empty() {
    let (k_in, k_last) = trim_frame_bounds(5_000, 5_000, 60);
    assert!(k_last >= k_in);
}

#[test]
fn audio_shift_uses_the_frame_floored_trim_in() {
    let (k_in, _) = trim_frame_bounds(1_234, 9_000, 30);
    let q = k_in * 1000 / 30;
    assert_eq!((k_in, q), (37, 1_233));
    assert_eq!(audio_shift_ms(Some(5_000), 4_000, q), 1_000 - 1_233);
    assert_eq!(audio_shift_ms(Some(5_000), 4_000, 1_000), 0);
    assert_eq!(audio_shift_ms(None, 4_000, 1_000), -1_000);
}

#[test]
fn untrimmed_last_index_matches_the_old_total_out_formula() {
    let full_dur_ms = 12_345u32;
    let old_total_out = (full_dur_ms as u64 * 60) / 1000;
    let (k_in, k_last) = trim_frame_bounds(0, full_dur_ms, 60);
    assert_eq!((k_in, k_last), (0, old_total_out));
}
