//! Smart typing duration for a manual zoom (owner, 2026-09-14: "when typing is happening the zoom
//! duration adjusts accordingly"). A zoom marked `smart_typing` has its `end_ms` refitted by
//! `edit::commands::apply_edit_op` whenever it is switched on or its start moves: the end lands
//! one hold after the last key of the typing chain that begins at (or just after) the start, so
//! the zoom stays on the field for as long as the words keep coming and lets go once they stop.
//! The doc always carries a concrete `end_ms`, so the timeline, the preview and the export need
//! no new path - this is a fit, not a live mode.
use crate::edit::model::EditDoc;
use crate::edit::ops::region::dur_bound;
use crate::export::remap::TimeMap;
use crate::session::paths::ProjectPaths;

/// Where a smart zoom starting at `start_ms` ends, given ascending keystroke times on the SAME
/// clock: the first key within `hold_ms` of the start opens a chain, each further key within
/// `hold_ms` of the previous extends it, and the end is the last key plus `hold_ms`, capped at
/// `dur_ms`. `None` when no key falls within `hold_ms` of the start (the zoom keeps its own end).
pub fn smart_end(start_ms: u32, keys: &[u32], hold_ms: u32, dur_ms: u32) -> Option<u32> {
    let hold = hold_ms.max(1);
    let mut last: Option<u32> = None;
    for &k in keys.iter().filter(|&&k| k >= start_ms) {
        let anchor = last.unwrap_or(start_ms);
        if k - anchor > hold { break; }
        last = Some(k);
    }
    last.map(|k| k.saturating_add(hold).min(dur_ms).max(start_ms.saturating_add(1)))
}

/// The recording's keystrokes on the doc's clock: `typing.json` (event time) shifted onto the clip
/// clock the way the seed shifts every region (`seed::output_shift`), then through the doc's own
/// cuts and speed spans (`TimeMap::out_of`) onto the output clock every zoom is on.
pub fn typing_on_doc_clock(paths: &ProjectPaths, doc: &EditDoc) -> Vec<u32> {
    let shift = crate::edit::seed::output_shift(paths);
    let map = TimeMap::build(&doc.trim, &doc.cuts, &doc.speed, doc.clip_ms.max(1));
    let mut keys: Vec<u32> = crate::events::track::typing::TypingLog::load(&paths.typing()).ms.iter()
        .map(|&t| map.out_of((t as i64 + shift).max(0) as u32)).collect();
    keys.sort_unstable();
    keys
}

/// Refit zoom `id`'s end to the typing after its start, if it is a smart-typing zoom and the
/// recording has typing there; otherwise leave it exactly as the op set it.
pub fn refit(doc: &mut EditDoc, id: &str, paths: &ProjectPaths) {
    let Some(z) = doc.zooms.iter().find(|z| z.id == id) else { return };
    if !z.smart_typing { return; }
    let (start, hold) = (z.start_ms, doc.settings.zoom.hold_ms);
    let dur = match dur_bound(doc) { u32::MAX => doc.clip_ms.max(start + 1), d => d };
    let keys = typing_on_doc_clock(paths, doc);
    if let Some(end) = smart_end(start, &keys, hold, dur) {
        if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) { z.end_ms = end; }
    }
}

#[cfg(test)]
mod tests {
    use super::smart_end;

    #[test]
    fn a_chain_of_keys_ends_one_hold_after_the_last() {
        assert_eq!(smart_end(1000, &[1500, 2200, 2900, 3400], 1800, 60_000), Some(5200));
    }

    #[test]
    fn a_gap_longer_than_the_hold_breaks_the_chain() {
        assert_eq!(smart_end(1000, &[1500, 2000, 9000, 9500], 1800, 60_000), Some(3800));
    }

    #[test]
    fn no_key_near_the_start_leaves_the_zoom_alone() {
        assert_eq!(smart_end(1000, &[5000, 5200], 1800, 60_000), None);
        assert_eq!(smart_end(1000, &[], 1800, 60_000), None);
        assert_eq!(smart_end(1000, &[200, 800], 1800, 60_000), None); // keys before the start do not count
    }

    #[test]
    fn the_end_is_capped_at_the_clip() {
        assert_eq!(smart_end(1000, &[1500, 2000], 1800, 3000), Some(3000));
    }
}
