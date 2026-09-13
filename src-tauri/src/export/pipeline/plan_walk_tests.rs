use super::*;

fn steps(plan: &[u64]) -> Vec<(u64, u64, u64)> {
    let mut c = PlanCursor::new(plan.to_vec());
    let mut v = Vec::new();
    while let Some(s) = c.next() { v.push((s.j, s.k, s.decodes_needed)); }
    v
}

#[test]
fn a_plain_plan_decodes_one_frame_per_output_frame_after_the_warm_up() {
    assert_eq!(steps(&[3, 4, 5]), vec![(0, 3, 4), (1, 4, 1), (2, 5, 1)]);
}

#[test]
fn a_cut_or_a_fast_span_skips_frames() {
    assert_eq!(steps(&[0, 2, 4, 40]), vec![(0, 0, 1), (1, 2, 2), (2, 4, 2), (3, 40, 36)]);
}

#[test]
fn slow_motion_reuses_the_held_frame() {
    assert_eq!(steps(&[7, 7, 8, 8]), vec![(0, 7, 8), (1, 7, 0), (2, 8, 1), (3, 8, 0)]);
}

#[test]
fn an_empty_plan_yields_nothing() {
    assert!(steps(&[]).is_empty());
    assert!(PlanCursor::new(vec![]).is_empty());
    assert_eq!(PlanCursor::new(vec![1, 2]).len(), 2);
}
