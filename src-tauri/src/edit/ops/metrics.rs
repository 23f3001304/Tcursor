use crate::edit::model::EditDoc;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Metrics {
    pub duration_ms: u32,
    pub kept_ms: u32,
    pub zoom_count: usize,
    pub cut_count: usize,
}

pub fn metrics(doc: &EditDoc) -> Metrics {
    let duration_ms = doc.trim.out_ms;
    let trim_in = doc.trim.in_ms;
    let trim_out = doc.trim.out_ms;
    let trim_span = trim_out.saturating_sub(trim_in);
    let cut_sum: u32 = doc
        .cuts
        .iter()
        .map(|c| {
            let s = c.start_ms.max(trim_in);
            let e = c.end_ms.min(trim_out);
            e.saturating_sub(s)
        })
        .sum();
    Metrics {
        duration_ms,
        kept_ms: trim_span.saturating_sub(cut_sum),
        zoom_count: doc.zooms.len(),
        cut_count: doc.cuts.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::ops::api::{apply, EditOp};

    #[test]
    fn metrics_kept_ms_subtracts_cuts() {
        let mut doc = EditDoc::default();
        apply(
            &mut doc,
            EditOp::SetTrim {
                in_ms: 0,
                out_ms: 10000,
            },
        );
        apply(
            &mut doc,
            EditOp::AddCut {
                start_ms: 1000,
                end_ms: 3000,
            },
        );
        let m = metrics(&doc);
        assert_eq!((m.duration_ms, m.kept_ms, m.cut_count), (10000, 8000, 1));
    }

    #[test]
    fn a_cut_outside_the_trim_window_does_not_reduce_kept_ms() {
        let mut doc = EditDoc::default();
        apply(
            &mut doc,
            EditOp::SetTrim {
                in_ms: 2000,
                out_ms: 6000,
            },
        );
        apply(
            &mut doc,
            EditOp::AddCut {
                start_ms: 8000,
                end_ms: 9000,
            },
        );
        assert_eq!(metrics(&doc).kept_ms, 4000);
        apply(
            &mut doc,
            EditOp::AddCut {
                start_ms: 1000,
                end_ms: 3000,
            },
        );
        assert_eq!(
            metrics(&doc).kept_ms,
            3000,
            "only the 1000ms inside the trim counts"
        );
    }
}
