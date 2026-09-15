use crate::edit::model::{Cut, EditDoc, Speed};
use crate::edit::ops::api::EditOp;
use crate::edit::ops::region::{clamp_order, dur_bound};
use crate::export::remap::{FACTOR_MAX, FACTOR_MIN};

fn next_id<'a>(ids: impl Iterator<Item = &'a str>, prefix: char, len: usize) -> String {
    let mut n = None;
    let mut count = 0usize;
    for id in ids {
        count += 1;
        if let Some(v) = id.strip_prefix(prefix).and_then(|d| d.parse::<u32>().ok()) {
            n = Some(n.map_or(v, |m: u32| m.max(v)));
        }
    }
    debug_assert_eq!(count, len);
    format!("{prefix}{}", n.map_or(len as u32, |m| m + 1))
}

pub(crate) fn next_cut_id(doc: &EditDoc) -> String {
    next_id(doc.cuts.iter().map(|c| c.id.as_str()), 'c', doc.cuts.len())
}
pub(crate) fn next_speed_id(doc: &EditDoc) -> String {
    next_id(
        doc.speed.iter().map(|s| s.id.as_str()),
        's',
        doc.speed.len(),
    )
}

fn merge_sorted(cuts: Vec<Cut>) -> Vec<Cut> {
    let mut out: Vec<Cut> = Vec::new();
    for c in cuts {
        match out.last_mut() {
            Some(l) if c.start_ms <= l.end_ms => l.end_ms = l.end_ms.max(c.end_ms),
            _ => out.push(c),
        }
    }
    out
}

pub fn normalize_cuts(doc: &mut EditDoc) {
    let hi = dur_bound(doc);
    let mut cuts = std::mem::take(&mut doc.cuts);
    for c in &mut cuts {
        c.start_ms = c.start_ms.min(hi);
        c.end_ms = c.end_ms.min(hi);
    }
    cuts.retain(|c| c.start_ms < c.end_ms);
    cuts.sort_by_key(|c| c.start_ms);
    doc.cuts = merge_sorted(cuts);
}

pub fn normalize_speed(doc: &mut EditDoc) {
    let hi = dur_bound(doc);
    let mut spans = std::mem::take(&mut doc.speed);
    for s in &mut spans {
        s.start_ms = s.start_ms.min(hi);
        s.end_ms = s.end_ms.min(hi);
        s.factor = s.factor.clamp(FACTOR_MIN as f32, FACTOR_MAX as f32);
    }
    spans.sort_by_key(|s| s.start_ms);
    let mut out: Vec<Speed> = Vec::new();
    for mut s in spans {
        if let Some(l) = out.last() {
            s.start_ms = s.start_ms.max(l.end_ms);
        }
        if s.start_ms < s.end_ms {
            out.push(s);
        }
    }
    doc.speed = out;
}

pub fn apply_time_op(doc: &mut EditDoc, op: &EditOp) -> bool {
    match op {
        EditOp::AddCut { start_ms, end_ms } => {
            let id = next_cut_id(doc);
            doc.cuts.push(Cut {
                id,
                start_ms: *start_ms,
                end_ms: *end_ms,
            });
            normalize_cuts(doc);
        }
        EditOp::AddCuts { spans } => {
            let mut incoming: Vec<Cut> = spans
                .iter()
                .filter(|(a, b)| a < b)
                .map(|&(a, b)| Cut {
                    id: String::new(),
                    start_ms: a,
                    end_ms: b,
                })
                .collect();
            incoming.sort_by_key(|c| c.start_ms);
            for mut c in merge_sorted(incoming) {
                c.id = next_cut_id(doc);
                doc.cuts.push(c);
            }
            normalize_cuts(doc);
        }
        EditOp::UpdateCut {
            id,
            start_ms,
            end_ms,
        } => {
            if let Some(c) = doc.cuts.iter_mut().find(|c| &c.id == id) {
                let (mut s, mut e) = (start_ms.unwrap_or(c.start_ms), end_ms.unwrap_or(c.end_ms));
                clamp_order(&mut s, &mut e, start_ms.is_some());
                c.start_ms = s;
                c.end_ms = e;
            }
            normalize_cuts(doc);
        }
        EditOp::RemoveCut { id } => {
            doc.cuts.retain(|c| &c.id != id);
        }
        EditOp::SetSpeed {
            start_ms,
            end_ms,
            factor,
        } => {
            let id = next_speed_id(doc);
            doc.speed.push(Speed {
                id,
                start_ms: *start_ms,
                end_ms: *end_ms,
                factor: *factor,
            });
            normalize_speed(doc);
        }
        EditOp::UpdateSpeed {
            id,
            start_ms,
            end_ms,
            factor,
        } => {
            if let Some(sp) = doc.speed.iter_mut().find(|s| &s.id == id) {
                let (mut s, mut e) = (start_ms.unwrap_or(sp.start_ms), end_ms.unwrap_or(sp.end_ms));
                clamp_order(&mut s, &mut e, start_ms.is_some());
                sp.start_ms = s;
                sp.end_ms = e;
                if let Some(f) = factor {
                    sp.factor = *f;
                }
            }
            normalize_speed(doc);
        }
        EditOp::RemoveSpeed { id } => {
            doc.speed.retain(|s| &s.id != id);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
#[path = "timeops_tests.rs"]
mod tests;
