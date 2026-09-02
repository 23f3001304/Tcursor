// The warm-preview cache: one live `FrameRenderer` per open recording, shared by every preview
// command (frame, background, camera track, layout(s), clicks, cursor kinds). Split out of
// `mod.rs` so both files stay under the line budget, and restructured so the cache cell's mutex
// is never held across a `FrameRenderer::new` or a render - the entry is OWNED by the caller for
// the duration of a call, the same take/put-back discipline `preview_fx.rs`'s `with_fx` uses for
// the FX renderer. See `session.md` for the full why (and why the build gate is separate).
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;
use crate::export::render::{FrameRenderer, RenderMeta};
use crate::export::types::Aspect;
use crate::session::paths::ProjectPaths;

/// A warm preview renderer cached for one recording + edit revision.
pub(crate) struct Cached { pub folder: String, pub mtime: Option<SystemTime>, pub aspect: Aspect, pub renderer: FrameRenderer, pub meta: RenderMeta }

/// A one-entry cache whose entry is handed OUT to the caller for the duration of a call: the
/// `cell` mutex is taken only to remove the entry and to put it back, never across `make`/`work`.
/// `gate` serializes callers instead (an entry is used `&mut`, so use is exclusive regardless),
/// which also collapses a cold-start stampede into a single build.
pub(crate) struct WarmSlot<T> { cell: Mutex<Option<T>>, gate: Mutex<()> }

impl<T> Default for WarmSlot<T> {
    fn default() -> Self { Self { cell: Mutex::new(None), gate: Mutex::new(()) } }
}

impl<T> WarmSlot<T> {
    /// Run `work` on the cached entry: `reuse` gets the cached entry (if any) and returns it
    /// refreshed, or `None` to discard it; `make` builds a replacement. Only `cell` is a cache
    /// lock and it is never held while `reuse`/`make`/`work` run.
    ///
    /// LOCK ORDERING: `reuse`/`make` (`reuse`/`build` below) call `edit::seed::load_or_seed`,
    /// which internally takes `edit::lock::doc_lock` for the span of any `edit.json` write it
    /// makes - so this establishes `gate -> doc_lock` ordering. See `edit::lock::doc_lock`'s doc
    /// comment for why the reverse never occurs (nothing under that lock ever touches `gate`,
    /// `cell`, or `generate_once`).
    pub(crate) fn with<R>(&self, reuse: impl FnOnce(T) -> Option<T>, make: impl FnOnce() -> Result<T, String>,
        work: impl FnOnce(&mut T) -> Result<R, String>) -> Result<R, String> {
        let _gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        let taken = self.cell.lock().unwrap_or_else(|e| e.into_inner()).take();
        let mut entry = match taken.and_then(reuse) { Some(e) => e, None => make()? };
        let out = work(&mut entry);
        *self.cell.lock().unwrap_or_else(|e| e.into_inner()) = Some(entry);
        out
    }

    /// Whether the cache cell is unlocked right now - the assertable form of "no expensive work
    /// runs under the cell lock" (`session_tests.rs` calls it from inside `make` and `work`).
    #[cfg(test)]
    pub(crate) fn cell_free(&self) -> bool { self.cell.try_lock().is_ok() }

    /// Whether an entry is cached right now (the slot holds at most one, by construction).
    #[cfg(test)]
    pub(crate) fn is_warm(&self) -> bool { self.cell.lock().unwrap_or_else(|e| e.into_inner()).is_some() }
}

/// Managed Tauri state: the most-recently-used warm preview renderer (one at a time), plus a
/// lock-free snapshot of the one fact about it the 25x/sec FX overlay needs.
#[derive(Default)]
pub struct PreviewSession { slot: WarmSlot<Cached>, webcam: AtomicBool }

impl PreviewSession {
    /// Whether the most-recently-warmed preview renderer's project has a recorded webcam - used
    /// by `preview_fx_overlay` to gate the spotlight's camera-exclusion hole the same way the
    /// export gates it (`has_webcam` in `fx_state.rs`/`render/mod.rs`). A relaxed atomic load, not
    /// a lock: this is called once per FX overlay request (~25/sec) and must never queue behind a
    /// frame render or a cold build. `false` (no hole) before anything has warmed the cache yet -
    /// fails safe, never an un-dimmed rectangle.
    pub fn has_webcam(&self) -> bool { self.webcam.load(Ordering::Relaxed) }
}

/// Refresh a cached entry so it can serve `folder` at `mtime`, or `None` when it cannot.
fn reuse(mut c: Cached, folder: &str, mtime: Option<SystemTime>, paths: &ProjectPaths) -> Option<Cached> {
    if c.folder != folder { return None; } // different recording - nothing is reusable
    if c.mtime == mtime { return Some(c); } // unchanged edit.json - the cheap hit, no JSON read
    // edit.json changed. A same-aspect edit refreshes the cheap edit-derived state in place; a
    // new aspect resizes the frame, so the GPU compositor/background/FX (sized for the OLD dims)
    // cannot just refresh and the entry has to be rebuilt from scratch.
    if c.aspect != crate::edit::seed::load_or_seed(paths).aspect { return None; }
    c.renderer.reload_edit(paths);
    c.mtime = mtime;
    Some(c)
}

/// Build a cold entry for `folder` - the expensive path (`FrameRenderer::new`: event-log decode,
/// ffprobe/ffmpeg subprocesses, wgpu pipelines, cursor-pack prep). Runs with no cache lock held.
fn build(folder: &str, mtime: Option<SystemTime>, paths: &ProjectPaths) -> Result<Cached, String> {
    let aspect = crate::edit::seed::load_or_seed(paths).aspect;
    let (renderer, meta) = super::build_renderer(paths).map_err(|e| e.to_string())?;
    Ok(Cached { folder: folder.to_string(), mtime, aspect, renderer, meta })
}

/// Run `f` with the warm renderer for `folder`, (re)building it when the folder or the doc's
/// aspect changes. The single place the preview cache is keyed - shared by every preview command
/// (frame, background, camera track, layouts, clicks, cursor kinds) so the warm-up logic lives
/// once. Every caller is an `async` command running on `spawn_blocking`, so nothing here is ever
/// on the main thread.
pub(crate) fn with_warm<T>(session: &PreviewSession, folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>) -> Result<T, String> {
    let paths = ProjectPaths { folder: PathBuf::from(folder) };
    let mtime = std::fs::metadata(paths.edit()).and_then(|m| m.modified()).ok();
    session.slot.with(
        |c| reuse(c, folder, mtime, &paths),
        || build(folder, mtime, &paths),
        |c| {
            // Refresh the lock-free snapshot `has_webcam` serves, on every warm-renderer touch.
            session.webcam.store(c.renderer.has_webcam(), Ordering::Relaxed);
            f(c, &paths)
        },
    )
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
