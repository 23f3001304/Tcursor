use super::*;
use crate::edit::model::EditDoc;
use crate::edit::ops::api::EditOp;
use crate::edit::text::{TextAnchor, TextAnim, TextKind, TextSize};

fn doc() -> EditDoc {
    let mut d = EditDoc::default();
    d.clip_ms = 10_000;
    d
}

fn add(d: &mut EditDoc, at_ms: u32, dur_ms: u32, kind: TextKind) -> String {
    apply_text(
        d,
        EditOp::AddText {
            at_ms,
            dur_ms,
            kind,
        },
    );
    d.texts.last().unwrap().id.clone()
}

fn upd(id: &str, fields: &str) -> EditOp {
    serde_json::from_str(&format!(r#"{{"op":"update_text","id":"{id}"{fields}}}"#)).unwrap()
}

#[test]
fn an_added_text_lands_inside_the_clip() {
    let mut d = doc();
    add(&mut d, 9_500, 2_000, TextKind::Title);
    assert_eq!((d.texts[0].start_ms, d.texts[0].end_ms), (9_500, 10_000));
    assert!(d.texts[0].id.starts_with('t'));
}

#[test]
fn each_kind_seeds_its_own_text_size_anchor_and_style() {
    let mut d = doc();
    add(&mut d, 0, 1000, TextKind::Title);
    add(&mut d, 0, 1000, TextKind::LowerThird);
    add(&mut d, 0, 1000, TextKind::Stat);
    add(&mut d, 0, 1000, TextKind::Callout);
    let t = &d.texts;
    assert_eq!(
        (
            t[0].text.as_str(),
            t[0].sub.as_deref(),
            t[0].size,
            t[0].pos,
            t[0].style.as_str()
        ),
        (
            "Your title",
            None,
            TextSize::L,
            TextAnchor::MidCenter,
            "clean"
        )
    );
    assert_eq!(
        (
            t[1].text.as_str(),
            t[1].sub.as_deref(),
            t[1].size,
            t[1].pos,
            t[1].style.as_str()
        ),
        (
            "Name",
            Some("Role"),
            TextSize::M,
            TextAnchor::BottomLeft,
            "bar"
        )
    );
    assert_eq!(
        (
            t[2].text.as_str(),
            t[2].sub.as_deref(),
            t[2].size,
            t[2].pos,
            t[2].style.as_str()
        ),
        (
            "128",
            Some("faster"),
            TextSize::Xl,
            TextAnchor::MidCenter,
            "clean"
        )
    );
    assert_eq!(
        (
            t[3].text.as_str(),
            t[3].sub.as_deref(),
            t[3].size,
            t[3].pos,
            t[3].style.as_str(),
            t[3].anim_in
        ),
        (
            "A callout",
            None,
            TextSize::S,
            TextAnchor::BottomCenter,
            "plate",
            TextAnim::Typewriter
        )
    );
    let ids: Vec<&str> = t.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["t0", "t1", "t2", "t3"]);
}

#[test]
fn update_clamps_the_span_and_orders_it() {
    let mut d = doc();
    let id = add(&mut d, 1000, 2000, TextKind::Title);
    apply_text(&mut d, upd(&id, r#","start_ms":4000"#));
    assert_eq!((d.texts[0].start_ms, d.texts[0].end_ms), (4000, 4000));
    apply_text(&mut d, upd(&id, r#","end_ms":50000"#));
    assert_eq!(d.texts[0].end_ms, 10_000);
}

#[test]
fn the_double_optional_sub_leaves_clears_and_sets() {
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::LowerThird);
    apply_text(&mut d, upd(&id, ""));
    assert_eq!(d.texts[0].sub.as_deref(), Some("Role"));
    apply_text(&mut d, upd(&id, r#","sub":null"#));
    assert_eq!(d.texts[0].sub, None);
    apply_text(&mut d, upd(&id, r#","sub":"  CEO  ""#));
    assert_eq!(d.texts[0].sub.as_deref(), Some("CEO"));
}

#[test]
fn text_is_trimmed_and_truncated_and_whitespace_is_kept_as_is() {
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::Title);
    let long = "x".repeat(MAX_TEXT_CHARS + 50);
    apply_text(&mut d, upd(&id, &format!(r#","text":"  {long}  ""#)));
    assert_eq!(d.texts[0].text.chars().count(), MAX_TEXT_CHARS);
    apply_text(&mut d, upd(&id, r#","text":"   ""#));
    assert_eq!(d.texts[0].text, "");
}

#[test]
fn valid_text_style_falls_back_to_clean() {
    for s in TEXT_STYLES {
        assert_eq!(valid_text_style(s), s);
    }
    assert_eq!(valid_text_style("neon"), "clean");
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::Title);
    apply_text(&mut d, upd(&id, r#","style":"neon""#));
    assert_eq!(d.texts[0].style, "clean");
}

#[test]
fn offset_is_clamped_per_component_and_a_non_finite_offset_is_dropped() {
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::Title);
    apply_text(&mut d, upd(&id, r#","offset":[0.9,-0.9]"#));
    assert_eq!(d.texts[0].offset, [0.5, -0.5]);
    apply_text(
        &mut d,
        EditOp::UpdateText {
            id: id.clone(),
            start_ms: None,
            end_ms: None,
            text: None,
            sub: None,
            kind: None,
            style: None,
            pos: None,
            offset: Some([f32::NAN, 0.0]),
            size: None,
            anim_in: None,
            anim_out: None,
            in_ms: None,
            out_ms: None,
            easing: None,
        },
    );
    assert_eq!(d.texts[0].offset, [0.5, -0.5]);
}

#[test]
fn animation_times_are_clamped_and_easing_is_coerced() {
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::Title);
    apply_text(
        &mut d,
        upd(&id, r#","in_ms":9000,"out_ms":0,"easing":"wobble""#),
    );
    assert_eq!((d.texts[0].in_ms, d.texts[0].out_ms), (4000, 0));
    assert_eq!(d.texts[0].easing, "smooth");
    apply_text(&mut d, upd(&id, r#","easing":"cubic(0.25,0.1,0.25,1)""#));
    assert_eq!(d.texts[0].easing, "cubic(0.250,0.100,0.250,1.000)");
}

#[test]
fn remove_text_drops_the_one_item_and_an_unknown_id_is_a_noop() {
    let mut d = doc();
    let id = add(&mut d, 0, 1000, TextKind::Title);
    add(&mut d, 0, 1000, TextKind::Stat);
    apply_text(&mut d, EditOp::RemoveText { id: "nope".into() });
    assert_eq!(d.texts.len(), 2);
    apply_text(&mut d, EditOp::RemoveText { id });
    assert_eq!(d.texts.len(), 1);
    assert_eq!(d.texts[0].kind, TextKind::Stat);
}
