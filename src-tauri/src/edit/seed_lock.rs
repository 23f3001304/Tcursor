// The locked write path behind `seed::load_or_seed` - split out of `seed.rs` (which was at the
// 200-line budget) so the two-phase locking shape (fast path / precompute-then-lock) has room to
// be documented properly. Bug-sweep-2 Task 7 round 2 (H3): a code-review pass on round 1 found
// that locking only `edit::commands`' three IPC entry points left `seed::load_or_seed`'s OWN
// internal seed/migrate/lift write unlocked everywhere else it's called from (~8 other call
// sites across `ai`/`export` - see `edit::lock::doc_lock`'s doc comment for the full list and the
// lock-ordering analysis vs. `WarmSlot::gate`).
use crate::edit::model::{EditDoc, EffectKind, DOC_VERSION};
use crate::session::paths::ProjectPaths;

/// Return an existing `edit.json`, else build+migrate+lift one and persist it - see `seed.rs`'s
/// `load_or_seed` re-export for the full "why self-locking, why some callers use
/// `load_or_seed_locked` instead" doc. Two phases:
///
/// 1. **FAST PATH** (no lock): an unlocked read that turns out already fully current returns
///    immediately - the common case once a project has been opened once. Nothing to write, so
///    nothing to race.
/// 2. **SLOW PATH**: precompute every ffprobe-backed input (`derive_seed_inputs`) BEFORE taking
///    `edit::lock::doc_lock`, then hand off to `load_or_seed_locked` for the actual
///    re-read-and-maybe-write under the lock. The lock is therefore only ever held across cheap
///    in-memory work plus one atomic save - never across a `build_timeline`/ffprobe call.
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc {
    let unlocked = EditDoc::load(&paths.edit());
    if let Some(doc) = &unlocked {
        if !needs_seed_write(doc) { return unlocked.unwrap(); }
    }
    let (precomputed_default, shift, true_dur) = derive_seed_inputs(paths, &unlocked);
    let lock = crate::edit::lock::doc_lock(paths);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    load_or_seed_locked(paths, precomputed_default, shift, true_dur)
}

/// The locked core of `load_or_seed`. **Caller MUST already hold `edit::lock::doc_lock(paths)`** -
/// this function takes no lock of its own, so `edit::commands::apply_edit_op` can call it as part
/// of one larger locked critical section that also covers its own `apply` + save (`std::sync::Mutex`
/// is NOT reentrant, so it must never call the self-locking `load_or_seed` above while already
/// holding this same lock). Re-reads `edit.json` fresh - authoritative, since a concurrent writer
/// may have changed it since any earlier unlocked snapshot, and that write must win - then applies
/// `migrate`/`lift_always_on_spotlight` and saves if either changed anything (or the doc was
/// freshly built). `precomputed_default`/`shift`/`true_dur` come from `derive_seed_inputs` - safe
/// to reuse here even though they may have been computed before this lock was taken, because they
/// depend only on the recording's STATIC artifact files, never on `edit.json`'s own content (see
/// that function's doc comment for why a value it decided NOT to compute can never turn out to
/// have been needed after all).
pub(crate) fn load_or_seed_locked(paths: &ProjectPaths, precomputed_default: Option<EditDoc>, shift: i64, true_dur: u32) -> EditDoc {
    let (mut doc, fresh) = match EditDoc::load(&paths.edit()) {
        Some(d) => (d, false),
        None => (precomputed_default.unwrap_or_else(|| crate::edit::seed::build_default(paths)), true),
    };
    let migrated = crate::edit::migrate::migrate(&mut doc, shift, true_dur);
    if crate::edit::ops::effects::lift_always_on_spotlight(&mut doc) || fresh || migrated {
        let _ = doc.save(&paths.edit());
    }
    doc
}

/// Precompute the (possibly ffprobe-backed) inputs `load_or_seed_locked` might need, from an
/// UNLOCKED snapshot of the doc (`None` if missing/corrupt) - safe to run before taking
/// `edit::lock::doc_lock` because none of them read `edit.json`: `build_default` only reads the
/// recording's OTHER artifact files (`settings.json`/`events.json`/`actions.json`/`typing.json`),
/// and `output_shift`/`true_duration_ms` only read `events.json`/`sync.json`. Gated by
/// `unlocked`'s own state so a pure spotlight-lift write (by far the most common non-fresh case -
/// flipping the Effects panel's Spotlight toggle) never pays for an unneeded `build_timeline`
/// pass.
///
/// **Why a gated-off value can never turn out to be needed under the lock (monotonicity):**
/// `doc.version` only ever increases (nothing decrements it) and `doc.clip_ms` only ever moves
/// from `0` to nonzero (nothing resets it) - so if the unlocked snapshot already had
/// `version >= 2` / `clip_ms != 0`, the freshly re-read doc under the lock is GUARANTEED to as
/// well; a gated-off value is only ever stale-and-UNUSED, never stale-and-insufficient. The one
/// theoretical exception is a doc that was corrupt at the unlocked read (which this function sees
/// as `None`, same as missing) and got fixed by something else before the lock - not reachable
/// through this app's own write paths. Even then, `migrate` treats `true_dur == 0` as its normal
/// "unknowable, don't guess" case (bails cleanly, matching `true_duration_ms`'s own contract), so
/// the affected backfill is simply deferred to a later `load_or_seed` call rather than wrong or
/// lost - the chosen "bail", not "loop", resolution for a re-check that (in practice, never)
/// invalidates the precomputed inputs.
pub(crate) fn derive_seed_inputs(paths: &ProjectPaths, unlocked: &Option<EditDoc>) -> (Option<EditDoc>, i64, u32) {
    let needs_shift = unlocked.as_ref().map_or(false, |d| d.version < 2);
    let needs_true_dur = unlocked.as_ref().map_or(false, |d| d.clip_ms == 0);
    let precomputed_default = if unlocked.is_none() { Some(crate::edit::seed::build_default(paths)) } else { None };
    let shift = if needs_shift { crate::edit::seed::output_shift(paths) } else { 0 };
    let true_dur = if needs_true_dur { crate::edit::seed::true_duration_ms(paths) } else { 0 };
    (precomputed_default, shift, true_dur)
}

/// Whether `load_or_seed` would need to write `doc` back to disk: an old-schema doc, one still
/// missing the `clip_ms` backfill, or one with the always-on spotlight toggle on but no lifted
/// region yet. Exactly mirrors the three conditions `migrate`/`lift_always_on_spotlight` check -
/// a doc this returns `false` for is genuinely a no-op read, so `load_or_seed`'s FAST PATH skips
/// the per-folder lock entirely for it.
fn needs_seed_write(doc: &EditDoc) -> bool {
    doc.version < DOC_VERSION
        || doc.clip_ms == 0
        || (doc.settings.clickfx.spotlight && !doc.effects.iter().any(|e| matches!(e.kind, EffectKind::Spotlight)))
}

#[cfg(test)]
#[path = "seed_lock_tests.rs"]
mod tests;
