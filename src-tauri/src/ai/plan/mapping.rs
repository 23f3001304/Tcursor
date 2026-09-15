use crate::ai::plan::json::extract_json;
use crate::ai::plan::schema::*;
use crate::edit::model::{EffectKind, ZoomTarget};
use crate::edit::ops::api::EditOp;
use crate::edit::ops::region::valid_layout;
use serde::Deserialize;

#[derive(Deserialize)]
struct PlanV2 {
    #[serde(default)]
    edits: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawEdit {
    kind: String,
    #[serde(default)]
    at_ms: u32,
    #[serde(default)]
    dur_ms: u32,
    #[serde(default)]
    rect: Option<[f32; 4]>,
    #[serde(default)]
    layout: Option<String>,
    #[serde(default)]
    factor: Option<f32>,
    #[serde(default)]
    in_ms: u32,
    #[serde(default)]
    out_ms: u32,
    #[serde(default)]
    why: String,
}

pub fn proposals_from_json(raw: &str, dur_ms: u32, clicks: &[ClickAt]) -> Vec<AiProposal> {
    let json = match extract_json(raw) {
        Some(j) => j,
        None => return Vec::new(),
    };
    let plan: PlanV2 = match serde_json::from_str(json) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<AiProposal> = plan
        .edits
        .into_iter()
        .filter_map(|v| serde_json::from_value::<RawEdit>(v).ok())
        .filter_map(|e| one(&e, dur_ms, clicks))
        .collect();
    out.sort_by_key(|p| p.at_ms);
    out = drop_overlaps(out);
    for (i, p) in out.iter_mut().enumerate() {
        p.id = format!("p{i}");
    }
    out
}

fn one(e: &RawEdit, total: u32, clicks: &[ClickAt]) -> Option<AiProposal> {
    let kind = ProposalKind::parse(&e.kind)?;
    if !RENDERABLE_KINDS.contains(&kind) {
        return None;
    }
    let why = tidy(&e.why);
    let rect = e.rect.filter(sane_rect);
    if kind == ProposalKind::Trim {
        return trim(e, total, why);
    }

    let (mut at, dur) = span(kind, e.at_ms, e.dur_ms, total)?;
    let mut keep_rect = rect;
    let ops = match kind {
        ProposalKind::Zoom => {
            let snap = snap_to_click(at, clicks);
            if let Some((t, ..)) = snap {
                at = t;
            }
            let scale = rect.map_or(2.0, |r| (1.0 / r[2].max(r[3])).clamp(MIN_SCALE, MAX_SCALE));
            let mut ops = vec![EditOp::AddZoomFull {
                at_ms: at,
                dur_ms: dur,
                scale,
            }];
            let point = rect
                .map(|r| (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0))
                .or_else(|| snap.map(|(_, x, y)| (x, y)));
            if let Some((x, y)) = point {
                ops.push(aim(x, y));
            }
            ops
        }
        ProposalKind::Layout => {
            let name = valid_layout(e.layout.as_deref().unwrap_or(""));
            if !matches!(name.as_str(), "presenter" | "camera" | "camera_only") {
                return None;
            }
            vec![EditOp::AddLayoutSeg {
                at_ms: at,
                dur_ms: dur,
                layout: name,
                transition_out_ms: None,
                easing_out: None,
            }]
        }
        ProposalKind::Spotlight => {
            let r = rect?;
            let radius = (r[2].max(r[3]) / 2.0).clamp(0.05, 0.40);
            vec![
                EditOp::AddEffect {
                    kind: EffectKind::Spotlight,
                    start_ms: at,
                    end_ms: at + dur,
                },
                EditOp::UpdateEffect {
                    id: NEW_ID.into(),
                    start_ms: None,
                    end_ms: None,
                    fade_in_ms: None,
                    fade_out_ms: None,
                    mode: None,
                    dim: None,
                    radius: Some(radius),
                    feather: None,
                    layer: None,
                },
            ]
        }
        ProposalKind::Cut => {
            keep_rect = None;
            vec![EditOp::AddCuts {
                spans: vec![(at, at + dur)],
            }]
        }
        ProposalKind::Speed => {
            keep_rect = None;
            let f = e
                .factor
                .filter(|f| f.is_finite() && (f - 1.0).abs() > 0.01)?;
            vec![EditOp::SetSpeed {
                start_ms: at,
                end_ms: at + dur,
                factor: f.clamp(0.25, 8.0),
            }]
        }
        ProposalKind::Trim => unreachable!("returned above"),
    };
    Some(AiProposal {
        id: String::new(),
        kind,
        why,
        at_ms: at,
        dur_ms: dur,
        rect: keep_rect,
        ops,
    })
}

fn aim(x: f32, y: f32) -> EditOp {
    EditOp::UpdateZoom {
        id: NEW_ID.into(),
        start_ms: None,
        end_ms: None,
        scale: None,
        target: Some(ZoomTarget::Fixed { x, y }),
        easing: None,
        zoom_in_ms: None,
        zoom_out_ms: None,
        layer: None,
        smart_typing: None,
        easing_out: None,
    }
}

fn span(kind: ProposalKind, at: u32, raw_dur: u32, total: u32) -> Option<(u32, u32)> {
    if at >= total || at.checked_add(raw_dur)? > total {
        return None;
    }
    let (lo, hi) = match kind {
        ProposalKind::Zoom | ProposalKind::Spotlight => (600, 6_000),
        ProposalKind::Layout | ProposalKind::Speed => (1_000, total),
        _ => (300, total),
    };
    let dur = raw_dur.clamp(lo.min(total), hi.min(total));
    if dur == 0 || at.checked_add(dur)? > total {
        return None;
    }
    Some((at, dur))
}

fn trim(e: &RawEdit, total: u32, why: String) -> Option<AiProposal> {
    let op = if e.out_ms == 0 && e.in_ms > 0 && e.in_ms < total {
        EditOp::SetTrim {
            in_ms: e.in_ms,
            out_ms: 0,
        }
    } else {
        let (in_ms, out_ms) = (e.in_ms.min(e.out_ms), e.out_ms.min(total));
        if out_ms <= in_ms {
            return None;
        }
        EditOp::SetTrim { in_ms, out_ms }
    };
    Some(AiProposal {
        id: String::new(),
        kind: ProposalKind::Trim,
        why,
        at_ms: 0,
        dur_ms: 0,
        rect: None,
        ops: vec![op],
    })
}

pub(crate) fn snap_to_click(at_ms: u32, clicks: &[ClickAt]) -> Option<(u32, f32, f32)> {
    clicks
        .iter()
        .min_by_key(|c| c.t_ms.abs_diff(at_ms))
        .filter(|c| c.t_ms.abs_diff(at_ms) <= SNAP_MS)
        .map(|c| (c.t_ms.saturating_sub(PRE_ROLL_MS), c.x, c.y))
}

fn sane_rect(r: &[f32; 4]) -> bool {
    r.iter().all(|v| v.is_finite())
        && r[2] > 0.02
        && r[3] > 0.02
        && r[0] >= -0.001
        && r[1] >= -0.001
        && r[0] + r[2] <= 1.001
        && r[1] + r[3] <= 1.001
}

fn tidy(why: &str) -> String {
    let flat = why.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= WHY_MAX {
        return flat;
    }
    let mut out = String::new();
    for w in flat.split(' ') {
        if out.chars().count() + w.chars().count() + 1 > WHY_MAX {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    if out.is_empty() {
        out.extend(flat.chars().take(WHY_MAX));
    }
    out
}

fn drop_overlaps(sorted: Vec<AiProposal>) -> Vec<AiProposal> {
    let mut kept: Vec<AiProposal> = Vec::new();
    for p in sorted {
        if !kept
            .iter()
            .any(|k| k.kind == p.kind && p.at_ms < k.at_ms + k.dur_ms.max(1))
        {
            kept.push(p);
        }
    }
    kept
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
