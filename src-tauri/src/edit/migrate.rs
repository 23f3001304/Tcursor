use crate::edit::model::{EditDoc, DOC_VERSION};
use crate::session::paths::ProjectPaths;

pub(crate) fn migrate(doc: &mut EditDoc, shift: i64, true_dur: u32) -> bool {
    let mut changed = false;
    if doc.version < DOC_VERSION {
        if doc.version < 2 {
            v1_to_v2(doc, shift);
        }
        doc.version = DOC_VERSION;
        changed = true;
    }
    if doc.clip_ms == 0 && true_dur > 0 {
        doc.clip_ms = true_dur;
        changed = true;
    }
    changed
}

fn v1_to_v2(doc: &mut EditDoc, shift: i64) {
    let mv = |t: &mut u32| *t = (*t as i64 + shift).max(0) as u32;
    for e in &mut doc.effects {
        mv(&mut e.start_ms);
        mv(&mut e.end_ms);
    }
    for l in &mut doc.layout {
        mv(&mut l.start_ms);
        mv(&mut l.end_ms);
    }
}

pub(crate) fn output_shift(paths: &ProjectPaths) -> i64 {
    let log = match crate::events::model::EventLog::load(&paths.events()) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    tl.events_ms as i64 - tl.frames.first().copied().unwrap_or(0) as i64
}

pub fn true_duration_ms(paths: &ProjectPaths) -> u32 {
    let log = match crate::events::model::EventLog::load(&paths.events()) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let vs = tl.frames.first().copied().unwrap_or(0);
    (tl.frames.last().copied().unwrap_or(vs).max(vs + 1) - vs) as u32
}
