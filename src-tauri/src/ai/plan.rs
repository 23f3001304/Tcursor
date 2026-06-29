use serde::Deserialize;
use crate::edit::api::EditOp;

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
    // Balance-count braces from first '{'
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut end = start;
    for (i, c) in s[start..].char_indices() {
        match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { end = start + i; break; } } _ => {} }
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
        let in_ms = t.in_ms.min(t.out_ms);
        let out_ms = t.out_ms.min(clip_len);
        if out_ms > in_ms {
            ops.push(EditOp::SetTrim { in_ms, out_ms });
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
}
