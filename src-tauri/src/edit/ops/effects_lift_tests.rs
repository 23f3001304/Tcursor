use super::*;

#[test]
fn lifts_always_on_spotlight_to_full_span_region_and_disables_toggle() {
    let mut doc = empty();
    doc.trim.out_ms = 8000;
    doc.settings.clickfx.spotlight = true;
    assert!(lift_always_on_spotlight(&mut doc));
    assert_eq!(doc.effects.len(), 1);
    assert!(matches!(doc.effects[0].kind, EffectKind::Spotlight));
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (0, 8000));
    assert!(!doc.settings.clickfx.spotlight);
    assert!(!lift_always_on_spotlight(&mut doc));
}

#[test]
fn lift_is_noop_when_spotlight_toggle_off() {
    let mut doc = empty();
    doc.settings.clickfx.spotlight = false;
    assert!(!lift_always_on_spotlight(&mut doc));
    assert!(doc.effects.is_empty());
}

#[test]
fn lift_is_noop_on_a_truly_unseeded_doc() {
    let mut doc = empty();
    doc.settings.clickfx.spotlight = true;
    assert!(!lift_always_on_spotlight(&mut doc));
    assert!(doc.effects.is_empty());
    assert!(doc.settings.clickfx.spotlight);
}

#[test]
fn lift_bounds_by_clip_ms_not_a_smaller_trim_out_ms() {
    let mut doc = empty();
    doc.clip_ms = 60_000;
    doc.trim.out_ms = 5_000;
    doc.settings.clickfx.spotlight = true;
    assert!(lift_always_on_spotlight(&mut doc));
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (0, 60_000)
    );
}

// H2, failure scenario B: `trim.out_ms == 0` is the ordinary "no trim / whole clip" sentinel
// (Reset Trim, or trimming out at the clip end) - once `clip_ms` is known, it must NOT be read
// as "degenerate doc, skip the lift". A fresh recording with no manual trim is exactly this
// state, and it's the state the UX audit caught playing entirely dimmed.
#[test]
fn lift_bounds_by_clip_ms_when_trim_out_ms_is_the_no_trim_sentinel() {
    let mut doc = empty();
    doc.clip_ms = 38_000;
    doc.trim.out_ms = 0;
    doc.settings.clickfx.spotlight = true;
    assert!(lift_always_on_spotlight(&mut doc));
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (0, 38_000)
    );
}

#[test]
fn lift_falls_back_to_trim_out_ms_when_clip_ms_is_unknown() {
    let mut doc = empty();
    doc.trim.out_ms = 12_000;
    doc.settings.clickfx.spotlight = true;
    assert!(lift_always_on_spotlight(&mut doc));
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (0, 12_000)
    );
}
