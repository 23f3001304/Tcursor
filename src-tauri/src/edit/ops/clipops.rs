use crate::edit::clip::Clip;
use crate::edit::model::EditDoc;
use crate::edit::ops::api::EditOp;
use crate::edit::ops::ids::next_clip_id;
use crate::edit::ops::region::dur_bound;
use crate::export::remap::TimeMap;

pub const MAX_TRANSITION_MS: u32 = 2000;

fn full_dur(doc: &EditDoc) -> Option<u32> {
    let d = dur_bound(doc);
    (d != u32::MAX).then_some(d)
}

fn split(doc: &mut EditDoc, at_ms: u32, full: u32) {
    if doc.clips.is_empty() {
        let (lo, hi) = doc.trim.resolve(full);
        if at_ms <= lo || at_ms >= hi {
            return;
        }
        let a = next_clip_id(doc);
        doc.clips.push(Clip {
            id: a,
            src_in_ms: lo,
            src_out_ms: at_ms,
            transition_in_ms: 0,
        });
        let b = next_clip_id(doc);
        doc.clips.push(Clip {
            id: b,
            src_in_ms: at_ms,
            src_out_ms: hi,
            transition_in_ms: 0,
        });
        return;
    }
    let Some(i) = doc
        .clips
        .iter()
        .position(|c| c.src_in_ms < at_ms && at_ms < c.src_out_ms)
    else {
        return;
    };
    let id = next_clip_id(doc);
    let right = Clip {
        id,
        src_in_ms: at_ms,
        src_out_ms: doc.clips[i].src_out_ms,
        transition_in_ms: 0,
    };
    doc.clips[i].src_out_ms = at_ms;
    doc.clips.insert(i + 1, right);
}

fn clamp_transition(doc: &EditDoc, i: usize, want: u32, full: u32) -> u32 {
    if i == 0 {
        return want.min(MAX_TRANSITION_MS);
    }
    let map = TimeMap::build(&doc.trim, &doc.cuts, &doc.speed, &doc.clips, full);
    let shorter = map.clip_out_ms(i - 1).min(map.clip_out_ms(i));
    want.min(MAX_TRANSITION_MS).min(shorter / 2)
}

pub fn apply_clip(doc: &mut EditDoc, op: EditOp) {
    let Some(full) = full_dur(doc) else {
        return;
    };
    match op {
        EditOp::SplitAt { at_ms } => split(doc, at_ms, full),
        EditOp::MoveClip { id, to_index } => {
            let Some(i) = doc.clips.iter().position(|c| c.id == id) else {
                return;
            };
            let c = doc.clips.remove(i);
            let to = to_index.min(doc.clips.len());
            doc.clips.insert(to, c);
        }
        EditOp::UpdateClip {
            id,
            src_in_ms,
            src_out_ms,
            transition_in_ms,
        } => {
            let Some(i) = doc.clips.iter().position(|c| c.id == id) else {
                return;
            };
            {
                let c = &mut doc.clips[i];
                if let Some(v) = src_in_ms {
                    c.src_in_ms = v.min(full);
                }
                if let Some(v) = src_out_ms {
                    c.src_out_ms = v.min(full);
                }
                if c.src_in_ms > c.src_out_ms {
                    std::mem::swap(&mut c.src_in_ms, &mut c.src_out_ms);
                }
            }
            if doc.clips[i].src_in_ms == doc.clips[i].src_out_ms {
                doc.clips.remove(i);
                return;
            }
            if let Some(v) = transition_in_ms {
                doc.clips[i].transition_in_ms = clamp_transition(doc, i, v, full);
            }
        }
        EditOp::RemoveClip { id } => {
            if doc.clips.len() > 1 {
                doc.clips.retain(|c| c.id != id);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "clipops_tests.rs"]
mod tests;
