use serde::Deserialize;
use crate::edit::ops::api::EditOp;

#[derive(Deserialize)]
struct PlanZoom { at_ms: u32, dur_ms: u32, #[serde(default)] scale: Option<f32> }

#[derive(Deserialize)]
struct PlanTrim { in_ms: u32, out_ms: u32 }

#[derive(Deserialize)]
struct Plan {
    #[serde(default)] zooms: Vec<PlanZoom>,
    #[serde(default)] trim: Option<PlanTrim>,
}

fn extract_json(raw: &str) -> Option<&str> {
    // Strip markdown fences
    let s = if let Some(_inner) = raw.find("```") {
        let after = &raw[raw.find("```").unwrap()..];
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(after.len());
        let body = &after[body_start..];
        if let Some(end) = body.find("```") { &body[..end] } else { body }
    } else { raw };
    // Balance-count braces from first '{', STRING-AWARE (M4): a `}`/`{` inside a JSON string
    // value (e.g. a chatty model's `"note"` field) is not structure and must not move `depth` -
    // `format: "json"` only constrains the reply to be valid JSON, not to this schema, so a
    // model is free to put brace characters in its own prose fields. Track whether we're inside a
    // string, toggling on an unescaped `"` and skipping the character right after a `\` so an
    // escaped quote can't end the string early.
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut end = start;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in s[start..].char_indices() {
        if in_string {
            if escaped { escaped = false; }
            else if c == '\\' { escaped = true; }
            else if c == '"' { in_string = false; }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => { depth -= 1; if depth == 0 { end = start + i; break; } }
            _ => {}
        }
    }
    if depth != 0 { return None; }
    Some(&s[start..=end])
}

pub fn ops_from_json(raw: &str, dur_ms: u32) -> Result<Vec<EditOp>, String> {
    let json = extract_json(raw).ok_or_else(|| "no JSON object found".to_string())?;
    let plan: Plan = serde_json::from_str(json)
        .map_err(|e| format!("parse error: {}", e))?;
    let clip_len = dur_ms;
    let mut ops: Vec<EditOp> = plan.zooms.into_iter().filter_map(|z| {
        let scale = z.scale.unwrap_or(2.0).clamp(1.0, 4.0);
        let zdur = z.dur_ms.clamp(200, clip_len.max(200));
        let end = z.at_ms.checked_add(zdur)?; // drop adversarially large spans, never overflow
        if z.at_ms >= clip_len || end > clip_len { return None; }
        Some(EditOp::AddZoomFull { at_ms: z.at_ms, dur_ms: zdur, scale })
    }).collect();
    if let Some(t) = plan.trim {
        // 0 is the "runs to true end" sentinel (TrimOverlay/useTrimActions' convention, not "zero
        // length") - a head-only trim {"in_ms":2000,"out_ms":0} must keep out_ms == 0, not collapse
        // through min(in_ms, out_ms) into a same-as-in-ms, zero-length range that then gets dropped.
        if t.out_ms == 0 && t.in_ms > 0 && t.in_ms < clip_len {
            ops.push(EditOp::SetTrim { in_ms: t.in_ms, out_ms: 0 });
        } else {
            let in_ms = t.in_ms.min(t.out_ms);
            let out_ms = t.out_ms.min(clip_len);
            if out_ms > in_ms {
                ops.push(EditOp::SetTrim { in_ms, out_ms });
            }
        }
    }
    if ops.is_empty() { return Err("no usable edits".into()); }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIP: u32 = 10_000;

    #[test]
    fn clean_json_yields_ops() {
        let raw = r#"{"zooms":[{"at_ms":500,"dur_ms":1000,"scale":2.5}]}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], EditOp::AddZoomFull { at_ms: 500, dur_ms: 1000, scale } if (*scale - 2.5).abs() < 0.001));
    }

    #[test]
    fn fenced_json_still_parses() {
        let raw = "```json\n{\"zooms\":[{\"at_ms\":100,\"dur_ms\":800}]}\n```";
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn scale_9_clamped_to_4() {
        let raw = r#"{"zooms":[{"at_ms":0,"dur_ms":500,"scale":9.0}]}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert!(matches!(&ops[0], EditOp::AddZoomFull { scale, .. } if (*scale - 4.0).abs() < 0.001));
    }

    #[test]
    fn zoom_past_clip_is_dropped() {
        let raw = r#"{"zooms":[{"at_ms":9900,"dur_ms":500}]}"#;
        let result = ops_from_json(raw, CLIP);
        assert!(result.is_err());
    }

    #[test]
    fn garbage_returns_err() {
        assert!(ops_from_json("hello world no json here", CLIP).is_err());
    }

    /// M4: a `}` inside a JSON string value (a chatty model explaining itself in a `"note"` field)
    /// must not be counted as the object's closing brace - the OUTER `}` at the very end is the
    /// real one.
    #[test]
    fn brace_inside_a_string_value_does_not_truncate_the_object() {
        let raw = r#"{"note":"skipping the idle stretch} at the start","zooms":[{"at_ms":500,"dur_ms":1200}]}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], EditOp::AddZoomFull { at_ms: 500, dur_ms: 1200, .. }), "{:?}", ops);
    }

    /// The mirror case: an unmatched `{` inside a string must not imbalance `depth` either -
    /// without string-awareness this would drive `depth` to never return to 0, yielding
    /// `Err("no JSON object found")` for perfectly valid JSON.
    #[test]
    fn unmatched_brace_inside_a_string_does_not_imbalance_depth() {
        let raw = r#"{"note":"a { without a match","zooms":[{"at_ms":0,"dur_ms":500}]}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn escaped_quote_inside_a_string_does_not_end_the_string_early() {
        let raw = r#"{"note":"a \" quote } inside","zooms":[{"at_ms":0,"dur_ms":500}]}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn valid_trim_maps_to_set_trim() {
        let raw = r#"{"zooms":[{"at_ms":0,"dur_ms":500}],"trim":{"in_ms":200,"out_ms":8000}}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert!(ops.iter().any(|o| matches!(o, EditOp::SetTrim { in_ms: 200, out_ms: 8000 })));
    }

    #[test]
    fn huge_dur_ms_is_dropped_not_overflow() {
        // adversarial dur_ms = u32::MAX must drop the zoom, never panic (debug overflow-check)
        let raw = r#"{"zooms":[{"at_ms":100,"dur_ms":4294967295}]}"#;
        assert!(ops_from_json(raw, CLIP).is_err());
    }

    #[test]
    fn head_trim_with_zero_out_ms_runs_to_true_end() {
        // The prompt's own example: {"in_ms":2000,"out_ms":0} means "trim the head, keep the
        // rest" - 0 is the "runs to true end" sentinel (TrimOverlay/useTrimActions convention),
        // not "collapse to zero-length".
        let raw = r#"{"zooms":[{"at_ms":0,"dur_ms":500}],"trim":{"in_ms":2000,"out_ms":0}}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert!(ops.iter().any(|o| matches!(o, EditOp::SetTrim { in_ms: 2000, out_ms: 0 })), "{:?}", ops);
    }

    #[test]
    fn zero_in_and_out_ms_trim_yields_no_trim_op() {
        // "not yet set" - the model made no trim decision at all.
        let raw = r#"{"zooms":[{"at_ms":0,"dur_ms":500}],"trim":{"in_ms":0,"out_ms":0}}"#;
        let ops = ops_from_json(raw, CLIP).unwrap();
        assert!(!ops.iter().any(|o| matches!(o, EditOp::SetTrim { .. })), "{:?}", ops);
    }
}
