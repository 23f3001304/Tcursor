use crate::edit::captions::{Caption, CaptionWord};
use crate::edit::model::{EditDoc, Trim};
use crate::export::remap::TimeMap;

fn span(map: &TimeMap, start_ms: u32, end_ms: u32) -> Option<(u32, u32)> {
    let (s, e) = (map.out_of(start_ms), map.out_of(end_ms));
    (e > s).then_some((s, e))
}

pub fn remap_doc(doc: &EditDoc, map: &TimeMap) -> EditDoc {
    let mut out = doc.clone();
    out.zooms = doc
        .zooms
        .iter()
        .filter_map(|z| {
            span(map, z.start_ms, z.end_ms).map(|(s, e)| {
                let mut z = z.clone();
                z.start_ms = s;
                z.end_ms = e;
                z
            })
        })
        .collect();
    out.layout = doc
        .layout
        .iter()
        .filter_map(|l| {
            span(map, l.start_ms, l.end_ms).map(|(s, e)| {
                let mut l = l.clone();
                l.start_ms = s;
                l.end_ms = e;
                l
            })
        })
        .collect();
    out.effects = doc
        .effects
        .iter()
        .filter_map(|f| {
            span(map, f.start_ms, f.end_ms).map(|(s, e)| {
                let mut f = f.clone();
                f.start_ms = s;
                f.end_ms = e;
                f
            })
        })
        .collect();
    out.captions = doc
        .captions
        .iter()
        .filter_map(|c| {
            span(map, c.start_ms, c.end_ms).map(|(s, e)| {
                let words = c
                    .words
                    .iter()
                    .filter_map(|w| {
                        span(map, w.start_ms, w.end_ms).map(|(ws, we)| CaptionWord {
                            start_ms: ws,
                            end_ms: we,
                            text: w.text.clone(),
                        })
                    })
                    .collect();
                Caption {
                    start_ms: s,
                    end_ms: e,
                    words,
                    ..c.clone()
                }
            })
        })
        .collect();
    out.camera_moves = doc
        .camera_moves
        .iter()
        .map(|m| {
            let mut m = m.clone();
            m.t_ms = map.out_of(m.t_ms);
            m
        })
        .collect();
    out.trim = Trim::default();
    out.cuts.clear();
    out.speed.clear();
    out.clip_ms = map.out_dur_ms();
    out
}

#[cfg(test)]
#[path = "remap_doc_tests.rs"]
mod tests;
