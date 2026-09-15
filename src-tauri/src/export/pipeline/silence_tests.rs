use super::*;

const LOG: &str = "[silencedetect @ 0x1] silence_start: 1.25\n[silencedetect @ 0x1] silence_end: 3.5 | silence_duration: 2.25\nsize=N/A time=00:00:09.00\n[silencedetect @ 0x1] silence_start: 8\n";

#[test]
fn parses_pairs_and_an_unterminated_start_runs_to_the_end() {
    assert_eq!(parse_silencedetect(LOG), vec![(1.25, 3.5), (8.0, f64::MAX)]);
    assert!(parse_silencedetect("frame=  10 fps=0.0\n").is_empty());
}

#[test]
fn seconds_become_clip_ms_through_the_track_shift_and_an_open_end_stays_open() {
    assert_eq!(to_clip_ms(&[(1.25, 3.5)], -200), vec![(1050, 3300)]);
    assert_eq!(to_clip_ms(&[(0.1, 0.5)], -200), vec![(0, 300)]);
    assert_eq!(to_clip_ms(&[(8.0, f64::MAX)], 0), vec![(8000, u32::MAX)]);
}

#[test]
fn a_stretch_is_silent_only_when_both_tracks_are() {
    assert_eq!(
        intersect(&[(1000, 4000), (8000, 9000)], &[(2000, 3000), (3500, 8500)]),
        vec![(2000, 3000), (3500, 4000), (8000, 8500)]
    );
    assert!(intersect(&[(0, 100)], &[(100, 200)]).is_empty());
}

#[test]
fn padding_shrinks_each_side_and_short_leftovers_are_dropped() {
    assert_eq!(
        pad_and_filter(
            vec![(1000, 3000), (5000, 5900), (8000, 20_000)],
            150,
            700,
            0,
            10_000
        ),
        vec![(1150, 2850), (8150, 9850)]
    );
    assert_eq!(
        pad_and_filter(vec![(0, u32::MAX)], 150, 700, 500, 9000),
        vec![(650, 8850)],
        "an open end is the trim-out"
    );
}
