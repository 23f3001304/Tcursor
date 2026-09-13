// Every region list of an EditDoc moved onto the OUTPUT clock through a `TimeMap`, so the renderer
// and the preview evaluate zooms, layouts, effects and camera moves in the time the viewer sees.
// Durations (`zoom_in_ms`, `transition_ms`, fades) are deliberately NOT scaled: that is the whole
// point of approach A in the time-remap design - a 350ms zoom-in stays 350ms of output inside a
// 2x span. Trim, cuts and speed are consumed and come back cleared. Mirrored by `src/lib/remapDoc.ts`.
use crate::edit::model::{EditDoc, Trim};
use crate::export::remap::TimeMap;

/// The output span of a clip span, or `None` when it collapses (entirely inside a cut).
fn span(map: &TimeMap, start_ms: u32, end_ms: u32) -> Option<(u32, u32)> {
    let (s, e) = (map.out_of(start_ms), map.out_of(end_ms));
    (e > s).then_some((s, e))
}

pub fn remap_doc(doc: &EditDoc, map: &TimeMap) -> EditDoc {
    let mut out = doc.clone();
    out.zooms = doc.zooms.iter().filter_map(|z| span(map, z.start_ms, z.end_ms)
        .map(|(s, e)| { let mut z = z.clone(); z.start_ms = s; z.end_ms = e; z })).collect();
    out.layout = doc.layout.iter().filter_map(|l| span(map, l.start_ms, l.end_ms)
        .map(|(s, e)| { let mut l = l.clone(); l.start_ms = s; l.end_ms = e; l })).collect();
    out.effects = doc.effects.iter().filter_map(|f| span(map, f.start_ms, f.end_ms)
        .map(|(s, e)| { let mut f = f.clone(); f.start_ms = s; f.end_ms = e; f })).collect();
    // A keyframe pinned to a removed moment lands on the cut's end, the frame the viewer sees next.
    out.camera_moves = doc.camera_moves.iter().map(|m| { let mut m = m.clone(); m.t_ms = map.out_of(m.t_ms); m }).collect();
    out.trim = Trim::default();
    out.cuts.clear();
    out.speed.clear();
    out.clip_ms = map.out_dur_ms();
    out
}

#[cfg(test)]
#[path = "remap_doc_tests.rs"]
mod tests;
