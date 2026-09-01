// `WarmSlot` unit tests - the preview cache's lock discipline and its "exactly one warm entry"
// invariant, exercised with a stand-in entry type so they need no project on disk and no GPU.
// (A real `Cached` holds a `FrameRenderer`, which cannot be built without a recording.)
use super::WarmSlot;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;

/// Stand-in for `Cached`: `key` plays the role of (folder, aspect) and `alive` counts how many
/// entries exist at once, so "the cache still returns ONE warm renderer" is directly assertable.
struct Entry { key: u32, alive: Arc<AtomicUsize> }

impl Entry {
    fn new(key: u32, alive: &Arc<AtomicUsize>) -> Self {
        alive.fetch_add(1, SeqCst);
        Self { key, alive: alive.clone() }
    }
}

impl Drop for Entry {
    fn drop(&mut self) { self.alive.fetch_sub(1, SeqCst); }
}

#[test]
fn builds_once_then_reuses_the_warm_entry() {
    let (slot, alive, builds) = (WarmSlot::default(), Arc::new(AtomicUsize::new(0)), AtomicUsize::new(0));
    for _ in 0..3 {
        let key = slot.with(
            |e: Entry| (e.key == 7).then_some(e),
            || { builds.fetch_add(1, SeqCst); Ok(Entry::new(7, &alive)) },
            |e| Ok(e.key),
        ).unwrap();
        assert_eq!(key, 7);
    }
    assert_eq!(builds.load(SeqCst), 1, "a warm entry must be reused, not rebuilt");
    assert_eq!(alive.load(SeqCst), 1);
    assert!(slot.is_warm());
}

#[test]
fn a_rejected_entry_is_replaced_not_accumulated() {
    let (slot, alive) = (WarmSlot::default(), Arc::new(AtomicUsize::new(0)));
    // Each round asks for a different key (the folder/aspect-changed path), so `reuse` rejects
    // and drops the cached entry before `make` builds its replacement.
    for key in [1u32, 2, 3] {
        let got = slot.with(
            |e: Entry| (e.key == key).then_some(e),
            || Ok(Entry::new(key, &alive)),
            |e| Ok(e.key),
        ).unwrap();
        assert_eq!(got, key);
        assert_eq!(alive.load(SeqCst), 1, "exactly one entry may be warm at a time");
    }
}

#[test]
fn the_cell_lock_is_never_held_across_reuse_make_or_work() {
    let (slot, alive) = (WarmSlot::default(), Arc::new(AtomicUsize::new(0)));
    // Cold: only `make` + `work` run. `cell_free()` is a `try_lock` on the cache cell, so it
    // fails if this very call still held the guard - which is exactly the old `with_warm` shape.
    slot.with(
        |e: Entry| Some(e),
        || { assert!(slot.cell_free(), "cell locked across the build"); Ok(Entry::new(1, &alive)) },
        |_| { assert!(slot.cell_free(), "cell locked across the work"); Ok(()) },
    ).unwrap();
    // Warm: `reuse` runs too.
    slot.with(
        |e: Entry| { assert!(slot.cell_free(), "cell locked across reuse"); Some(e) },
        || panic!("must not rebuild a reusable entry"),
        |_| Ok(()),
    ).unwrap();
}

#[test]
fn concurrent_callers_share_one_build_and_one_warm_entry() {
    let slot = Arc::new(WarmSlot::default());
    let alive = Arc::new(AtomicUsize::new(0));
    let builds = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..4).map(|_| {
        let (slot, alive, builds) = (slot.clone(), alive.clone(), builds.clone());
        std::thread::spawn(move || slot.with(
            |e: Entry| Some(e),
            || {
                builds.fetch_add(1, SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(10)); // widen the race window
                Ok(Entry::new(9, &alive))
            },
            |e| Ok(e.key),
        ).unwrap())
    }).collect();
    for t in threads { assert_eq!(t.join().unwrap(), 9); }
    assert_eq!(builds.load(SeqCst), 1, "racing callers must not each run a cold build");
    assert_eq!(alive.load(SeqCst), 1);
}

#[test]
fn a_failed_build_leaves_the_slot_empty_and_surfaces_the_error() {
    let slot: WarmSlot<Entry> = WarmSlot::default();
    let err = slot.with(|e| Some(e), || Err("cold build failed".to_string()), |e| Ok(e.key)).unwrap_err();
    assert_eq!(err, "cold build failed");
    assert!(!slot.is_warm(), "a failed build must not leave a half-built entry behind");
}

/// Run `f` with the panic hook silenced (the panic under test is expected). Mirrors
/// `preview_fx_tests.rs`'s `a_panic_inside_the_render_does_not_disable_later_overlays`.
fn quiet_panic<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    out
}

/// A panic during a COLD BUILD must not wedge the cache for the rest of the session.
///
/// `gate` is the one lock held across `make`, so a panic there drops its guard mid-unwind and
/// **poisons it**. `with` recovers (`unwrap_or_else(|e| e.into_inner())`) instead of propagating;
/// swap that for `.unwrap()` and the next line of this test panics on the poison - and in the real
/// app every preview command (frame, camera track, layout, clicks, background) would fail forever
/// after one transient `FrameRenderer::new` fault. `FrameRenderer::new` genuinely can panic: it
/// spawns subprocesses, decodes logs and builds wgpu pipelines.
#[test]
fn a_panic_in_the_build_does_not_wedge_the_slot() {
    let (slot, alive) = (WarmSlot::default(), Arc::new(AtomicUsize::new(0)));
    let boom = quiet_panic(|| slot.with(
        |e: Entry| Some(e),
        || -> Result<Entry, String> { panic!("simulated cold-build failure") },
        |e| Ok(e.key),
    ));
    assert!(boom.is_err(), "the panic must propagate to the caller, not be swallowed");
    assert!(!slot.is_warm(), "a panicking build must leave nothing cached");

    // The real regression: this used to be reachable only if `gate` were not poison-recovering.
    let key = slot.with(|e: Entry| Some(e), || Ok(Entry::new(4, &alive)), |e| Ok(e.key)).unwrap();
    assert_eq!(key, 4, "the next caller must still get a working renderer");
    assert_eq!(alive.load(SeqCst), 1);
}

/// Same guarantee for a panic during the WORK (a render), against an already-warm slot - the
/// `GpuFx`-style transient-fault case `with_fx` was hardened for. The entry is a local by then, so
/// it is dropped during the unwind and the slot self-heals by rebuilding cold on the next call
/// (deliberately the same trade `with_fx` makes: drop it rather than cache a renderer that just
/// faulted). Asserted explicitly so a future change to that policy is a visible decision.
#[test]
fn a_panic_in_the_work_leaves_the_slot_rebuildable() {
    let (slot, alive, builds) = (WarmSlot::default(), Arc::new(AtomicUsize::new(0)), AtomicUsize::new(0));
    let warm = || slot.with(
        |e: Entry| Some(e),
        || { builds.fetch_add(1, SeqCst); Ok(Entry::new(5, &alive)) },
        |e| Ok(e.key),
    );
    assert_eq!(warm().unwrap(), 5);
    assert!(slot.is_warm());

    let boom = quiet_panic(|| slot.with(
        |e: Entry| Some(e),
        || -> Result<Entry, String> { panic!("must not rebuild a reusable entry") },
        |_: &mut Entry| -> Result<u32, String> { panic!("simulated render failure") },
    ));
    assert!(boom.is_err(), "the panic must propagate to the caller, not be swallowed");
    assert!(!slot.is_warm(), "the faulted entry is dropped, not cached");
    assert_eq!(alive.load(SeqCst), 0, "the unwind must not leak the entry either");

    assert_eq!(warm().unwrap(), 5, "the next caller must still get a working renderer");
    assert_eq!(builds.load(SeqCst), 2, "it rebuilds cold rather than reusing the faulted entry");
    assert_eq!(alive.load(SeqCst), 1);
}
