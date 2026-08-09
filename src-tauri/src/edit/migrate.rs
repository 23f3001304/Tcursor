// Doc-version migrations + the event/output clock shift they need, split out of seed.rs so that
// file stays under the size limit. `seed::load_or_seed` runs `migrate` right after load/build,
// before `effects::lift_always_on_spotlight`.
use crate::edit::model::{EditDoc, DOC_VERSION};
use crate::session::paths::ProjectPaths;

/// Bring a loaded doc up to `DOC_VERSION`, backfilling `clip_ms` along the way; returns whether it
/// changed (the caller then saves). One `if doc.version < N` step per schema bump, applied in
/// order, so a later version adds a step without touching the ones before it. The version is
/// ALWAYS stamped at the end, even when a step could not do its work (see `v1_to_v2`) - a
/// half-migrated doc must never be migrated twice. The `clip_ms` backfill runs on EVERY doc
/// (not gated by `version`), because a doc already at `DOC_VERSION` written before `clip_ms`
/// existed also has it at its zero default - it is a field backfill, not a schema bump, so it
/// gets its own `DOC_VERSION`-independent step.
pub(crate) fn migrate(doc: &mut EditDoc, paths: &ProjectPaths) -> bool {
    let mut changed = false;
    if doc.version < DOC_VERSION {
        if doc.version < 2 { v1_to_v2(doc, paths); }
        doc.version = DOC_VERSION;
        changed = true;
    }
    if doc.clip_ms == 0 {
        let true_dur = true_duration_ms(paths);
        if true_dur > 0 { doc.clip_ms = true_dur; changed = true; }
    }
    changed
}

/// v1 -> v2: v1 seeded `effects`/`layout` straight off the recorded action log (EVENT clock) while
/// its zooms were already output-time, so those regions sat ~`video_start - events_ms` (≈800 ms)
/// early against the editor timeline. Shift both onto the output clock, saturating at 0. When the
/// timeline cannot be rebuilt the shift is unknowable, so regions are left where they are.
fn v1_to_v2(doc: &mut EditDoc, paths: &ProjectPaths) {
    let shift = output_shift(paths);
    let mv = |t: &mut u32| *t = (*t as i64 + shift).max(0) as u32;
    for e in &mut doc.effects { mv(&mut e.start_ms); mv(&mut e.end_ms); }
    for l in &mut doc.layout { mv(&mut l.start_ms); mv(&mut l.end_ms); }
}

/// Ms to ADD to an event-clock timestamp to put it on the output clock (`events_ms - video_start`,
/// negative for a real recording, whose first video frame lands after the input trackers start).
/// 0 when `events.json` cannot be loaded: with no timeline the offset is unknowable, so the caller
/// must leave timestamps where they are rather than guess. Re-exported as `seed::output_shift` -
/// the AI director transcript (`ai::commands::build_plan`) reuses this SAME recipe rather than
/// re-deriving it, so the transcript and the seeded regions can never disagree on the shift.
pub(crate) fn output_shift(paths: &ProjectPaths) -> i64 {
    let log = match crate::events::model::EventLog::load(&paths.events()) { Ok(l) => l, Err(_) => return 0 };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    tl.events_ms as i64 - tl.frames.first().copied().unwrap_or(0) as i64
}

/// The recording's TRUE full duration (ms) - `video_end - video_start` from the real capture
/// timeline, independent of any user `trim.out_ms` selection. Preview helpers that need "how
/// long is the whole clip" (proxy re-timing in `ensure_proxy`, filmstrip thumbnail spacing in
/// `ensure_thumbs`) call this rather than reading `trim.out_ms` - which, once a user actually
/// trims, no longer means the recording's length. Also the source of truth `migrate` backfills
/// `EditDoc::clip_ms` from. Returns 0 if `events.json` cannot be loaded.
pub fn true_duration_ms(paths: &ProjectPaths) -> u32 {
    let log = match crate::events::model::EventLog::load(&paths.events()) { Ok(l) => l, Err(_) => return 0 };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let vs = tl.frames.first().copied().unwrap_or(0);
    (tl.frames.last().copied().unwrap_or(vs).max(vs + 1) - vs) as u32
}
