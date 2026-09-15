use crate::edit::captions::{Caption, CaptionWord};
use crate::edit::model::EditDoc;
use crate::edit::ops::api::EditOp;
use crate::edit::ops::ids::next_caption_id;
use crate::edit::ops::region::{clamp_order, dur_bound};

pub const MIN_CAPTION_MS: u32 = 200;

fn text_of(words: &[CaptionWord]) -> String {
    words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn enforce_min(c: &mut Caption, dur: u32, start_was_set: bool) {
    if c.end_ms > c.start_ms {
        return;
    }
    if start_was_set || c.end_ms < MIN_CAPTION_MS {
        c.end_ms = c
            .start_ms
            .saturating_add(MIN_CAPTION_MS)
            .min(dur.max(MIN_CAPTION_MS));
        c.start_ms = c.end_ms - MIN_CAPTION_MS;
    } else {
        c.start_ms = c.end_ms - MIN_CAPTION_MS;
    }
}

pub fn split_text(text: &str, frac: f32) -> (String, String) {
    let t = text.trim();
    let target = (frac.clamp(0.0, 1.0) * t.len() as f32).round() as usize;
    let Some(at) = t
        .char_indices()
        .filter(|(_, ch)| *ch == ' ')
        .map(|(i, _)| i)
        .min_by_key(|i| i.abs_diff(target))
    else {
        return (t.to_string(), String::new());
    };
    (t[..at].trim().to_string(), t[at + 1..].trim().to_string())
}

pub fn split_caption(c: &Caption, at_ms: u32, next_id: &str) -> Option<(Caption, Caption)> {
    if at_ms <= c.start_ms || at_ms >= c.end_ms {
        return None;
    }
    let (lw, rw): (Vec<CaptionWord>, Vec<CaptionWord>) =
        c.words.iter().cloned().partition(|w| w.start_ms < at_ms);
    let (lt, rt) = if c.words.is_empty() {
        let frac = (at_ms - c.start_ms) as f32 / (c.end_ms - c.start_ms) as f32;
        split_text(&c.text, frac)
    } else {
        (text_of(&lw), text_of(&rw))
    };
    Some((
        Caption {
            id: c.id.clone(),
            start_ms: c.start_ms,
            end_ms: at_ms,
            text: lt,
            words: lw,
        },
        Caption {
            id: next_id.to_string(),
            start_ms: at_ms,
            end_ms: c.end_ms,
            text: rt,
            words: rw,
        },
    ))
}

pub fn merge_captions(a: &Caption, b: &Caption) -> Caption {
    let text = match (a.text.trim(), b.text.trim()) {
        ("", t) | (t, "") => t.to_string(),
        (x, y) => format!("{x} {y}"),
    };
    let mut words = a.words.clone();
    words.extend(b.words.iter().cloned());
    Caption {
        id: a.id.clone(),
        start_ms: a.start_ms.min(b.start_ms),
        end_ms: a.end_ms.max(b.end_ms),
        text,
        words,
    }
}

pub fn apply_caption(doc: &mut EditDoc, op: EditOp) {
    let dur = dur_bound(doc);
    match op {
        EditOp::UpdateCaption {
            id,
            start_ms,
            end_ms,
            text,
        } => {
            let Some(c) = doc.captions.iter_mut().find(|c| c.id == id) else {
                return;
            };
            if let Some(v) = start_ms {
                c.start_ms = v.min(dur);
            }
            if let Some(v) = end_ms {
                c.end_ms = v.min(dur);
            }
            clamp_order(&mut c.start_ms, &mut c.end_ms, start_ms.is_some());
            if start_ms.is_some() || end_ms.is_some() {
                enforce_min(c, dur, start_ms.is_some());
            }
            if let Some(t) = text {
                if t.trim() != text_of(&c.words) {
                    c.words.clear();
                }
                c.text = t;
            }
        }
        EditOp::RemoveCaption { id } => doc.captions.retain(|c| c.id != id),
        EditOp::ClearCaptions => doc.captions.clear(),
        EditOp::SetCaptions { captions } => doc.captions = captions,
        EditOp::MergeCaptions { id } => {
            let Some(i) = doc.captions.iter().position(|c| c.id == id) else {
                return;
            };
            if i + 1 >= doc.captions.len() {
                return;
            }
            let merged = merge_captions(&doc.captions[i], &doc.captions[i + 1]);
            doc.captions.remove(i + 1);
            doc.captions[i] = merged;
        }
        EditOp::SplitCaption { id, at_ms } => {
            let Some(i) = doc.captions.iter().position(|c| c.id == id) else {
                return;
            };
            let next = next_caption_id(doc);
            let Some((a, b)) = split_caption(&doc.captions[i], at_ms, &next) else {
                return;
            };
            doc.captions[i] = a;
            doc.captions.insert(i + 1, b);
        }
        _ => return,
    }
    doc.captions.sort_by_key(|c| c.start_ms);
}

#[cfg(test)]
#[path = "captions_tests.rs"]
mod tests;
