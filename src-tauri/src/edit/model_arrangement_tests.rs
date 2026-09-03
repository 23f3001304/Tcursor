// Serde back-compat tests for `LayoutSeg.arrangement` (T34 L1 step 1), split out of
// model_tests.rs so that file stays under the size limit. The whole contract is ADDITIVE:
// an `edit.json` written before arrangements existed must load unchanged AND re-save
// byte-identically, so upgrading the app never rewrites a user's doc.
use super::*;

/// The exact bytes a pre-arrangement build wrote for one layout segment.
const OLD_SEG: &str = r#"{"id":"l0","start_ms":0,"end_ms":1000,"layout":"camera","transition_ms":350,"easing":"smooth","transition_out_ms":0,"easing_out":"smooth"}"#;

#[test]
fn old_layout_seg_json_loads_with_no_arrangement() {
    let s: LayoutSeg = serde_json::from_str(OLD_SEG).unwrap();
    assert_eq!(s.arrangement, None);
    assert_eq!(s.layout, "camera");
    assert_eq!((s.transition_ms, s.transition_out_ms), (350, 0));
}

/// `skip_serializing_if` is the byte-stability guarantee: loading an old doc and saving it back
/// must produce the SAME bytes, not a doc that has grown a new `"arrangement":null` key.
#[test]
fn an_untouched_old_seg_re_serializes_byte_identically() {
    let s: LayoutSeg = serde_json::from_str(OLD_SEG).unwrap();
    let out = serde_json::to_string(&s).unwrap();
    assert_eq!(out, OLD_SEG, "re-saving an untouched old segment must not change a byte");
}

/// The same guarantee one level up: a whole `EditDoc` round-trips through save/load with the
/// arrangement key absent from every segment.
#[test]
fn a_whole_old_doc_round_trips_without_growing_an_arrangement_key() {
    let doc = sample_doc();
    assert!(doc.layout.iter().all(|s| s.arrangement.is_none()));
    let json = serde_json::to_string(&doc).unwrap();
    assert!(!json.contains("arrangement"), "unset arrangement must not be written: {}", json);
    let p = tmp_path("edit_model_arrangement_backcompat.json");
    doc.save(&p).unwrap();
    assert_eq!(EditDoc::load(&p).unwrap(), doc);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn a_set_arrangement_round_trips_through_json() {
    let mut s: LayoutSeg = serde_json::from_str(OLD_SEG).unwrap();
    s.arrangement = Some(Arrangement {
        screen: Some(PanelPose { cx: 0.25, cy: 0.5, size: 0.8 }),
        cam: Some(PanelPose { cx: 0.75, cy: 0.5, size: 0.4 }),
    });
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"arrangement\""), "set arrangement must be written: {}", json);
    assert_eq!(serde_json::from_str::<LayoutSeg>(&json).unwrap(), s);
}

/// A hidden panel is `null` on the wire, NOT an omitted key - "not shown" is a real, saved
/// choice, so it must survive a round trip distinctly from "no arrangement at all".
#[test]
fn a_hidden_panel_round_trips_as_an_explicit_null() {
    let mut s: LayoutSeg = serde_json::from_str(OLD_SEG).unwrap();
    s.arrangement = Some(Arrangement { screen: None, cam: Some(PanelPose { cx: 0.5, cy: 0.5, size: 0.9 }) });
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains(r#""screen":null"#), "hidden panel must be an explicit null: {}", json);
    let back: LayoutSeg = serde_json::from_str(&json).unwrap();
    assert_eq!(back.arrangement.unwrap().screen, None);
    assert_eq!(back, s);
}

/// A partially-written arrangement (only one key present) still loads - the missing panel reads
/// as hidden, matching the `None` = "not shown" rule, rather than failing the whole doc.
#[test]
fn a_doc_carrying_an_arrangement_loads_through_edit_doc() {
    let seg = r#"{"id":"l0","start_ms":0,"end_ms":1000,"layout":"presenter","transition_ms":350,"easing":"smooth","arrangement":{"screen":{"cx":0.5,"cy":0.5,"size":0.7},"cam":null}}"#;
    let doc: EditDoc = serde_json::from_str(&format!(
        r#"{{"version":2,"trim":{{"in_ms":0,"out_ms":5000}},"cuts":[],"zooms":[],"speed":[],"layout":[{seg}],"settings":{{}}}}"#)).unwrap();
    let a = doc.layout[0].arrangement.unwrap();
    assert_eq!(a.screen, Some(PanelPose { cx: 0.5, cy: 0.5, size: 0.7 }));
    assert_eq!(a.cam, None);
    // The `layout` name survives as provenance even though the arrangement now drives resolution.
    assert_eq!(doc.layout[0].layout, "presenter");
}
