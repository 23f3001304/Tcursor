# src-tauri/src/export/preview/session.rs

The warm-preview cache: one live `FrameRenderer` per open recording, shared by every preview command (`preview_frame`, `preview_bg`, `camera_track`, `preview_layout`, `preview_layouts`, `click_track`, `cursor_kinds`). Split out of `mod.rs` so both files stay under the 200-line budget, and restructured (sweep-2 Task 1) so the cache cell's mutex is never held across a `FrameRenderer::new` or a render.

**Why the restructure.** The old `with_warm` took `session.0.lock()` and held that guard across the *whole* call - including the cold-build branch (`FrameRenderer::new`: event-log gzip decode, up to three `ffprobe`/`ffmpeg` subprocess spawns, two wgpu pipeline builds, cursor-pack prep) and across the closure body (which for `preview_frame` blocks on two `RawDecoder` ffmpeg subprocesses). Every command that took that lock was itself synchronous, so the lock and the work were both on the UI thread. Two things fall out of that: the window froze for the build, and `PreviewSession::has_webcam()` - taken by every `preview_fx_overlay`, ~25x/sec - contended on the same mutex. Making the commands `async` fixes the main thread; this file fixes the lock.

## Cached

```rust
pub(crate) struct Cached { pub folder: String, pub mtime: Option<SystemTime>, pub aspect: Aspect, pub renderer: FrameRenderer, pub meta: RenderMeta }
```

One warm preview renderer plus the key it is cached under: the project `folder`, `edit.json`'s `mtime` (the cheapest possible edit-revision stamp - one `metadata()` syscall, no JSON parse) and the doc's `aspect`. `renderer`/`meta` are what the preview commands actually use.

## WarmSlot

```rust
pub(crate) struct WarmSlot<T> { cell: Mutex<Option<T>>, gate: Mutex<()> }
```

A one-entry cache whose entry is handed *out* to the caller for the duration of a call. Two mutexes with two different jobs:

- **`cell`** - the cache itself. Locked only to remove the entry and to put it back; never across a build, a refresh, or the work. This is the mutex the "no expensive work under the cache lock" rule is about, and `cell_free()` makes it directly assertable from inside those callbacks.
- **`gate`** - serializes callers. An entry is used `&mut` (`FrameRenderer::step_camera`/`reload_edit`/`composite_at` all mutate), so exactly one caller may hold it at a time regardless; a gate is the honest way to express that, and it has a second payoff: N callers racing a cold cache collapse into **one** build instead of each running `FrameRenderer::new` on its own. That case is not hypothetical - the editor fires six `with_warm` commands on the same mount tick, and now that they are all `async` they land on six different blocking-pool threads. (The pure "double-checked insert, drop the loser's build" shape has no gate and would let all six build; `Default::default()` on the slot is the only construction, so the gate costs one uncontended lock on the warm path.)

`Default` is implemented by hand (not derived) so `T` needs no `Default` bound.

## WarmSlot::with

```rust
pub(crate) fn with<R>(&self, reuse: impl FnOnce(T) -> Option<T>, make: impl FnOnce() -> Result<T, String>,
    work: impl FnOnce(&mut T) -> Result<R, String>) -> Result<R, String>
```

Runs `work` on the cached entry.

### Inputs (what, and why it is needed)

- `reuse` - receives the cached entry **by value** and returns it (optionally refreshed in place) if it can serve this call, or `None` to discard it. *Why by value:* it is the ownership handoff that lets the cell lock be released before any of this runs.
- `make` - builds a replacement when there was no entry or `reuse` rejected it. *Why a closure:* the expensive path stays at the call site (`build`), outside this file's locking concerns, and runs with no cell lock held.
- `work` - the actual command body, against the live entry.

### Returns

`Result<R, String>` - whatever `work` returned, or `make`'s error. A `make` failure propagates with `?`, which leaves the slot **empty** (no half-built entry is cached); the next call simply rebuilds. A `work` error still puts the entry back, so one failed frame does not cost a rebuild.

### Implementation

1. Take `gate` (held for the whole body).
2. Lock `cell`, `take()` the entry, drop the guard - the cell is now free.
3. `taken.and_then(reuse)`, else `make()?`. A rejected entry is dropped inside `reuse`, so at most one entry exists at any moment.
4. Run `work` on `&mut entry`.
5. Lock `cell` again and put the entry back.

### Panic behaviour (and why every lock recovers rather than unwraps)

Both locks use `unwrap_or_else(|e| e.into_inner())`, matching `with_fx`/`recorder.rs`. The two mutexes sit on opposite sides of that line, and it is worth being exact about which one the recovery is actually load-bearing for:

- **`cell` cannot be poisoned by `reuse`/`make`/`work`.** Each of its guards is created and dropped inside a single statement, so no cell guard is ever alive while a callback runs. The recovery there is pure defence in depth.
- **`gate` can be, and is.** It is held across all three callbacks by design, so a panic in any of them drops its guard mid-unwind and poisons it. With `.unwrap()` the *next* `with_warm` call would panic on that poison and keep doing so forever - one transient fault inside `FrameRenderer::new` (subprocess spawns, log decode, wgpu pipeline builds) or inside a render would permanently disable every preview command for the rest of the session. That is exactly the failure `with_fx` was hardened against, on the other cache; `unwrap_or_else(|e| e.into_inner())` is what stops it here.

A panic also **empties the slot**: the entry is a local by the time `work` runs, so the unwind drops it and the next call rebuilds cold rather than re-caching a renderer that just faulted - deliberately the same trade `with_fx` makes.

`session_tests.rs` pins all of this: `a_panic_in_the_build_does_not_wedge_the_slot` and `a_panic_in_the_work_leaves_the_slot_rebuildable` each raise a panic, assert it propagates, assert the slot is empty and nothing leaked, then assert a subsequent caller still gets a working entry. Both fail if `gate`'s recovery is swapped for `.unwrap()`.

## WarmSlot::cell_free

```rust
#[cfg(test)]
pub(crate) fn cell_free(&self) -> bool
```

Whether the cache cell is unlocked right now (`try_lock`). Test-only, and the point of it: called from *inside* `reuse`/`make`/`work` it fails if that call still held the guard - which is exactly the old `with_warm` shape - so "the mutex is never held across a build" is a real assertion rather than a claim (`session_tests.rs`).

## WarmSlot::is_warm

```rust
#[cfg(test)]
pub(crate) fn is_warm(&self) -> bool
```

Whether an entry is cached right now. Test-only; the slot holds at most one entry by construction (`Option<T>`), so the tests pair this with a drop-counting entry type to assert the *count*.

## PreviewSession

```rust
#[derive(Default)]
pub struct PreviewSession { slot: WarmSlot<Cached>, webcam: AtomicBool }
```

Managed Tauri state (registered in `lib.rs`): the most-recently-used warm preview renderer, one at a time, plus a lock-free snapshot of the one fact about it the FX overlay needs.

## PreviewSession::has_webcam

```rust
pub fn has_webcam(&self) -> bool
```

Whether the most-recently-warmed preview renderer's project has a recorded webcam - used by `preview_fx_overlay` (`preview_fx.rs`) to gate the spotlight's camera-exclusion hole the same way the export gates it (`has_webcam` in `fx_state.rs`/`render/mod.rs`).

**A relaxed atomic load, not a lock (sweep-2 Task 1).** It used to be `self.0.lock()...renderer.has_webcam()` on the same mutex `with_warm` held across cold builds and frame renders, and it is called once per FX overlay request - ~25x/sec for the whole of playback and every scrub - so it could queue behind a multi-second renderer build. `with_warm` now stores `renderer.has_webcam()` into `webcam` on every warm-renderer touch, and this reads that snapshot.

Semantics are otherwise unchanged: it reports whichever project was warmed most recently, regardless of `folder`, which is what every preview command already implicitly relies on (the editor keeps at most one project's renderer warm, refreshed by `camera_track`/`preview_layout`/etc. on essentially every render, so by the time an FX overlay is requested the warm renderer already belongs to the open project). Returns `false` (no hole) before anything has warmed the cache - fails safe, never an un-dimmed rectangle.

## reuse

```rust
fn reuse(mut c: Cached, folder: &str, mtime: Option<SystemTime>, paths: &ProjectPaths) -> Option<Cached>
```

Refreshes a cached entry so it can serve `folder` at `mtime`, or returns `None` when it cannot. Checks in cost order:

1. **Different `folder`** -> `None`. Nothing is reusable, and (unlike the old code) no `edit.json` is read to find that out.
2. **Same `mtime`** -> `Some(c)` unchanged. The common hit: one `metadata()` syscall, no JSON parse.
3. Otherwise `edit.json` changed, so read the doc's `aspect` (`edit::seed::load_or_seed`). A **different aspect** resizes the frame - the cached GPU compositor, background and FX are all sized for the OLD dims and cannot just refresh - so `None`, forcing a rebuild. The **same aspect** (a zoom/spotlight/settings edit) calls the cheap `FrameRenderer::reload_edit` in place, restamps `mtime`, and returns the entry. A full rebuild is ~seconds; the in-place refresh is ~microseconds, which is what keeps editing snappy.

## build

```rust
fn build(folder: &str, mtime: Option<SystemTime>, paths: &ProjectPaths) -> Result<Cached, String>
```

Builds a cold entry: reads the doc's `aspect` (so the entry is keyed by what it was actually built for) and calls `mod.rs`'s `build_renderer`, i.e. `FrameRenderer::new` - the expensive path (event-log decode, ffprobe/ffmpeg subprocesses, wgpu pipelines, cursor-pack prep). Runs as `WarmSlot::with`'s `make`, so no cache lock is held while it does any of that.

## with_warm

```rust
pub(crate) fn with_warm<T>(session: &PreviewSession, folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>) -> Result<T, String>
```

Runs `f` with the warm renderer for `folder`, (re)building it when the folder or the doc's aspect changes. The single place the preview cache is keyed - shared by every preview command so the warm-up logic lives exactly once. The closure receives the cached renderer plus the resolved `ProjectPaths`; its return value is owned.

Every caller is an `async` command running under `tauri::async_runtime::spawn_blocking`, so nothing in here - build, refresh, or `f` - is ever on the main thread.

### Implementation

Resolves `paths` and stamps `edit.json`'s `mtime`, then hands three closures to `session.slot.with`: `reuse`, `build`, and a wrapper around `f` that first stores `renderer.has_webcam()` into the session's atomic (keeping `has_webcam`'s lock-free snapshot current on every warm-renderer touch).
