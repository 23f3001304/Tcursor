use crate::edit::model::EditDoc;
use crate::edit::ops::region::dur_bound;
use crate::export::remap::TimeMap;
use crate::session::paths::ProjectPaths;

pub fn smart_end(start_ms: u32, keys: &[u32], hold_ms: u32, dur_ms: u32) -> Option<u32> {
    let hold = hold_ms.max(1);
    let mut last: Option<u32> = None;
    for &k in keys.iter().filter(|&&k| k >= start_ms) {
        let anchor = last.unwrap_or(start_ms);
        if k - anchor > hold {
            break;
        }
        last = Some(k);
    }
    last.map(|k| {
        k.saturating_add(hold)
            .min(dur_ms)
            .max(start_ms.saturating_add(1))
    })
}

pub fn typing_on_doc_clock(paths: &ProjectPaths, doc: &EditDoc) -> Vec<u32> {
    let shift = crate::edit::seed::output_shift(paths);
    let map = TimeMap::build(&doc.trim, &doc.cuts, &doc.speed, doc.clip_ms.max(1));
    let mut keys: Vec<u32> = crate::events::track::typing::TypingLog::load(&paths.typing())
        .ms
        .iter()
        .map(|&t| map.out_of((t as i64 + shift).max(0) as u32))
        .collect();
    keys.sort_unstable();
    keys
}

pub fn refit(doc: &mut EditDoc, id: &str, paths: &ProjectPaths) {
    let Some(z) = doc.zooms.iter().find(|z| z.id == id) else {
        return;
    };
    if !z.smart_typing {
        return;
    }
    let (start, hold) = (z.start_ms, doc.settings.zoom.hold_ms);
    let dur = match dur_bound(doc) {
        u32::MAX => doc.clip_ms.max(start + 1),
        d => d,
    };
    let keys = typing_on_doc_clock(paths, doc);
    if let Some(end) = smart_end(start, &keys, hold, dur) {
        if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) {
            z.end_ms = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::smart_end;

    #[test]
    fn a_chain_of_keys_ends_one_hold_after_the_last() {
        assert_eq!(
            smart_end(1000, &[1500, 2200, 2900, 3400], 1800, 60_000),
            Some(5200)
        );
    }

    #[test]
    fn a_gap_longer_than_the_hold_breaks_the_chain() {
        assert_eq!(
            smart_end(1000, &[1500, 2000, 9000, 9500], 1800, 60_000),
            Some(3800)
        );
    }

    #[test]
    fn no_key_near_the_start_leaves_the_zoom_alone() {
        assert_eq!(smart_end(1000, &[5000, 5200], 1800, 60_000), None);
        assert_eq!(smart_end(1000, &[], 1800, 60_000), None);
        assert_eq!(smart_end(1000, &[200, 800], 1800, 60_000), None);
    }

    #[test]
    fn the_end_is_capped_at_the_clip() {
        assert_eq!(smart_end(1000, &[1500, 2000], 1800, 3000), Some(3000));
    }
}
