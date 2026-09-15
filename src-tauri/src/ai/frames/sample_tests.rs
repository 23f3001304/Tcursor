use super::*;

fn at(v: &[(u32, FrameReason)]) -> Vec<FrameAt> {
    v.iter()
        .map(|&(t_ms, reason)| FrameAt { t_ms, reason })
        .collect()
}

#[test]
fn a_clip_with_nothing_in_it_still_samples_the_idle_grid() {
    assert_eq!(
        sample_times(&[], &[], 30_000, MAX_FRAMES),
        at(&[
            (0, FrameReason::Idle),
            (8_000, FrameReason::Idle),
            (16_000, FrameReason::Idle),
            (24_000, FrameReason::Idle)
        ])
    );
}

#[test]
fn a_click_suppresses_the_idle_grid_point_it_sits_near() {
    assert_eq!(
        sample_times(&[3_100], &[], 12_000, MAX_FRAMES),
        at(&[(3_100, FrameReason::Click), (8_000, FrameReason::Idle)])
    );
}

#[test]
fn clicks_closer_than_the_minimum_gap_collapse_to_the_first() {
    let got = sample_times(&[5_000, 5_150, 5_300, 9_000], &[], 20_000, MAX_FRAMES);
    let clicks: Vec<u32> = got
        .iter()
        .filter(|f| f.reason == FrameReason::Click)
        .map(|f| f.t_ms)
        .collect();
    assert_eq!(
        clicks,
        vec![5_000, 9_000],
        "a double-click is one moment, not three frames"
    );
}

#[test]
fn a_click_wins_over_a_layout_switch_at_the_same_instant() {
    let got = sample_times(&[4_000], &[4_000], 20_000, MAX_FRAMES);
    assert_eq!(got.iter().filter(|f| f.t_ms == 4_000).count(), 1);
    assert!(got
        .iter()
        .any(|f| f.t_ms == 4_000 && f.reason == FrameReason::Click));
}

#[test]
fn more_events_than_the_cap_are_spread_and_never_exceed_it() {
    let clicks: Vec<u32> = (0..60).map(|i| i * 1_000).collect();
    let got = sample_times(&clicks, &[], 61_000, MAX_FRAMES);
    assert_eq!(got.len(), MAX_FRAMES);
    assert_eq!(got[0].t_ms, 0, "the first event is always kept");
    assert_eq!(got[MAX_FRAMES - 1].t_ms, 59_000, "and so is the last");
    assert!(
        got.windows(2).all(|w| w[0].t_ms < w[1].t_ms),
        "ascending and unique: {:?}",
        got
    );
}

#[test]
fn real_events_are_kept_before_idle_when_the_cap_bites() {
    let clicks: Vec<u32> = (0..MAX_FRAMES as u32).map(|i| i * 1_000).collect();
    let got = sample_times(&clicks, &[], 300_000, MAX_FRAMES);
    assert!(
        got.iter().all(|f| f.reason == FrameReason::Click),
        "a five-minute clip's idle grid must not crowd out what actually happened: {:?}",
        got
    );
}

#[test]
fn nothing_is_sampled_from_a_zero_length_clip() {
    assert!(sample_times(&[], &[], 0, MAX_FRAMES).is_empty());
    assert!(
        sample_times(&[5_000], &[], 0, MAX_FRAMES).is_empty(),
        "an event past the clip end is not a frame"
    );
}
