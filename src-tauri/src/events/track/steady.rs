pub const SHAPE_HOLD_MS: u32 = 100;

pub fn steady<T: Copy + PartialEq>(samples: &[(u32, T)], hold_ms: u32) -> Vec<(u32, T)> {
    let n = samples.len();
    let mut out: Vec<(u32, T)> = Vec::with_capacity(n);
    for (i, &(t, v)) in samples.iter().enumerate() {
        let lasts = i + 1 == n || samples[i + 1].0.saturating_sub(t) >= hold_ms;
        if (i == 0 || lasts) && out.last().map(|l| l.1) != Some(v) {
            out.push((t, v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flicker_between_two_shapes_is_dropped_and_the_real_dwell_keeps_its_start() {
        let s = [
            (0, 'a'),
            (1000, 'i'),
            (1030, 'a'),
            (1060, 'i'),
            (1090, 'a'),
            (1120, 'i'),
            (5000, 'a'),
        ];
        assert_eq!(steady(&s, 100), vec![(0, 'a'), (1120, 'i'), (5000, 'a')]);
    }

    #[test]
    fn a_flicker_that_ends_where_it_started_leaves_no_trace() {
        let s = [(0, 'a'), (1000, 'i'), (1030, 'a'), (1060, 'i'), (1090, 'a')];
        assert_eq!(steady(&s, 100), vec![(0, 'a')]);
    }

    #[test]
    fn a_change_that_lasts_exactly_the_hold_is_kept() {
        let s = [(0, 'a'), (500, 'h'), (600, 'a')];
        assert_eq!(steady(&s, 100), vec![(0, 'a'), (500, 'h'), (600, 'a')]);
    }

    #[test]
    fn the_first_sample_is_the_base_even_when_it_is_short() {
        let s = [(0, 'h'), (20, 'a'), (900, 'i')];
        assert_eq!(steady(&s, 100), vec![(0, 'h'), (20, 'a'), (900, 'i')]);
    }

    #[test]
    fn empty_and_single_tracks_pass_through() {
        assert_eq!(steady::<char>(&[], 100), vec![]);
        assert_eq!(steady(&[(7, 'a')], 100), vec![(7, 'a')]);
    }

    #[test]
    fn a_zero_hold_only_collapses_repeats() {
        let s = [(0, 'a'), (10, 'a'), (20, 'i'), (25, 'i')];
        assert_eq!(steady(&s, 0), vec![(0, 'a'), (20, 'i')]);
    }
}
