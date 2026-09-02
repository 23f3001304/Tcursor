# src-tauri/src/session/record/pause_totals.rs

Exact-span paused-time ledger for the WHOLE recording: the recorder's pause/resume commands stamp it, every input tracker (mouse, keyboard actions, typing, cursor-type) reads it through `stamp`, and since the C1/H2 fix both capture paths read it through `pause_clock.rs`. One ledger means `sync.json`, `video.mp4`'s own PTS, the audio WAVs and every event stream are compressed by exactly the same number and cannot drift apart.

It is stamped directly by the code path that flips the recorder's `paused` flag, because that path knows the exact wall-clock instant the toggle happens. `PauseClock` used to infer the span from the gaps between *paused frame arrivals* instead; since WGC only delivers frames on content change, that measured a pause over a static desktop as 0 ms and one over a busy desktop as ~100%, making the take's reported duration a function of what the screen happened to be doing (finding H2).

## PauseState

```rust
struct PauseState {
    paused_ms: u64,
    pause_started: u64,
}
```

Private. The two fields as ONE `Mutex`-guarded unit, not two independent atomics. *Why not two `AtomicU64`s:* `resume` clearing `pause_started` and folding its span into `paused_ms` must be a single visible step - a tracker thread is never serialized against `resume_recording` (clicking Resume is itself a mouse event the hook observes at that same instant), so a reader landing between two independent atomic writes could see "not paused" paired with the PRE-update `paused_ms`, under-counting by up to the whole just-closed span. The `Mutex` makes that torn read structurally impossible.

- `paused_ms: u64` - total duration of CLOSED pause episodes, in the recorder's `Clock` domain.
- `pause_started: u64` - wall-clock ms the current pause began, or `0` if not currently paused. *Why 0 as sentinel:* the recorder's `Clock` starts at creation and a pause at literal ms 0 is not a realistic scenario, so `0` unambiguously means "not paused".

## PauseTotals

```rust
pub struct PauseTotals {
    state: Mutex<PauseState>,
}
```

Accumulates paused wall-clock time from the exact pause/resume instants and exposes the elapsed-paused span, so a tracker's stamp site - or `PauseClock`, on behalf of either capture path - can subtract it from a raw timestamp. `pause`, `resume`, and `elapsed_paused` each take the lock once per call and observe/mutate the pair together; the lock is uncontended except right at a pause/resume boundary, so a hook callback taking it briefly is fine (`MouseTracker`'s hook already locks a `Mutex` per event for `SINK`).

- `state: Mutex<PauseState>` - the combined-state contract described above.

### Used by

- `src-tauri/src/session/record/recorder.rs` - `Running` holds one `Arc<PauseTotals>`; `pause_recording` / `resume_recording` stamp it from `r.clock.now_ms()` under the same lock that flips `r.paused`.
- `src-tauri/src/events/track/tracker.rs` - `MouseTracker`'s hook thread holds a clone; `hook_proc` calls `stamp` on every raw event time before pushing to the collector.
- `src-tauri/src/actions/keyboard.rs` - `KeyboardTracker`'s poll thread holds a clone; both the hotkey-action and typing stamp sites call `stamp`.
- `src-tauri/src/events/track/cursortracker.rs` - `CursorTypeTracker`'s poll thread holds a clone; every shape-change sample is stamped through it.

## PauseTotals::new

```rust
pub fn new() -> Self
```

Returns a fresh ledger with zero accumulated paused time and no in-progress pause.

## PauseTotals::pause

```rust
pub fn pause(&self, now_ms: u64)
```

Record a pause starting at `now_ms`.

### Inputs

- `now_ms: u64` - the recorder clock's reading at the instant `paused` is set `true`.

### Implementation

Locks `state` once. Idempotent: if a pause is already in progress (`pause_started != 0`), this is a no-op - it does NOT move the anchor forward, so a duplicate `pause()` call can't shorten the measured span. Otherwise stores `now_ms` into `pause_started`.

## PauseTotals::resume

```rust
pub fn resume(&self, now_ms: u64)
```

Close the in-progress pause as of `now_ms`, folding its span into `paused_ms`.

### Inputs

- `now_ms: u64` - the recorder clock's reading at the instant `paused` is cleared.

### Implementation

Locks `state` once and updates both fields under that single acquisition - clearing `pause_started` and adding to `paused_ms` are one atomic step, not two, which is the fix for the torn-read race described under `PauseState` above. Idempotent: if no pause is in progress (`pause_started == 0`), this is a no-op - no double-count from a stray `resume()` call. Otherwise resets `pause_started` to `0` and adds `now_ms - pause_started` (saturating) to `paused_ms`.

## PauseTotals::elapsed_paused

```rust
pub fn elapsed_paused(&self, now_ms: u64) -> u64
```

The total paused span as of `now_ms`: every closed episode plus, if a pause is currently in progress, the in-progress span up to `now_ms`.

### Returns

`paused_ms` if not currently paused; otherwise `paused_ms + (now_ms - pause_started)` (saturating). Both fields are read from a single lock acquisition, so this always sees a consistent pair - never a `pause_started` from one moment paired with a `paused_ms` from another.

## PauseTotals::stamp_ms

```rust
pub fn stamp_ms(&self, raw_ms: u64) -> u64
```

Pause-adjust a raw wall-clock reading: `raw_ms.saturating_sub(elapsed_paused(raw_ms))`. The single place the "a pause never happened" rule lives - the input trackers reach it through `stamp`, and both capture paths reach it through `PauseClock::tick` - so `sync.json`, `video.mp4`'s own PTS and every event stream are compressed by exactly the same number. Before the C1 fix the video paths had their own frame-arrival-based accumulator, which is how the video kept a paused span the rest of the recording had dropped.

### Inputs

- `raw_ms: u64` - a raw wall-clock reading in the recorder clock's domain (a `Clock::now_ms()` value, or a frame's already-clock-derived `ts`).

### Returns

The same instant with all paused time removed. Saturating, so a reading from before the ledger's first pause can never underflow.

## PauseTotals::stamp

```rust
pub fn stamp(&self, raw_ms: u64) -> u32
```

`stamp_ms` in the input trackers' narrower `t` type (ms since tracker start). Every input tracker calls this at its stamp site instead of using the raw value directly, so recorded events land in the same (pause-compressed) timeline as the video and audio clocks.

### Inputs

- `raw_ms: u64` - the tracker's own raw elapsed-ms reading (e.g. `Instant::elapsed()` since the tracker's thread started), treated as equivalent to the recorder clock's domain (both are wall-clock-derived from ~the same startup instant, the same assumption `events_ms` already relies on).

### Returns

`stamp_ms(raw_ms)` cast to `u32`. Events that arrive DURING a pause are not dropped - they collapse onto the pause boundary rather than landing at their true (parked) offset, which is an accepted behavior per the task brief (dropping would be a separate behavior change).

### Behaviors

- `mid_pause_elapsed_is_the_in_progress_span`: paused at 1000, `elapsed_paused(2500) == 1500` (in-progress span only).
- `closed_episode_elapsed_is_the_full_span`: paused 1000, resumed 3000, `elapsed_paused(3100) == 2000`.
- `resume_updates_both_fields_as_one_atomic_unit`: pins the invariant the two-atomic layout could tear on - `pause(1000); resume(3000)` then `elapsed_paused(3000) == 2000`. Single-threaded (a deterministic cross-thread interleaving test is impractical); the guarantee that no reader can observe the pair half-updated is structural (the `Mutex` in `PauseState`), not something a timing test could prove or this test exercises directly.
- `accumulates_across_multiple_pause_episodes`: two episodes (500ms, 300ms) sum to 800ms.
- `pause_is_idempotent_when_already_paused`: a second `pause()` call while already paused does not shorten the measured span.
- `resume_is_idempotent_when_not_paused`: a stray `resume()` call with no pause in progress does not corrupt `paused_ms`.
