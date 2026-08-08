# src-tauri/src/session/record/pause_clock.rs

Shared paused-time accumulator used by both capture paths (`gpu_record::Cap` and `recording_session::RecordingSession`). VFR capture only ticks on frame arrival - there is no dedicated "check for resume" moment - so the accumulator works purely from a stream of per-frame `(now, paused)` samples, unlike `session::pacing::run_paced`'s tight 2ms poll loop which can read the clock at the exact instant it notices a resume.

## PauseClock

```rust
#[derive(Default)]
pub struct PauseClock {
    paused_ms: u64,
    last_paused_at: Option<u64>,
}
```

Accumulates paused wall-clock time from a stream of per-frame `(now, paused)` samples and shifts capture timestamps to exclude it, so a pause leaves no gap between `sync.json` and `video.mp4`.

- `paused_ms: u64` - total paused time accumulated so far, subtracted from every timestamp recorded while not paused.
- `last_paused_at: Option<u64>` - the previous paused sample's `now`, or `None` if the last `observe` call was unpaused (or this is the first call). *Why needed:* consecutive paused samples measure elapsed time as a delta against each other, not against a fixed pause-start; this field is the "other end" of that delta.

### Used by

- `src-tauri/src/session/record/gpu_record.rs` - `Cap` holds one instance, calling `observe` once per `on_frame_arrived`.
- `src-tauri/src/session/record/recording_session.rs` - `RecordingSession` holds one instance, calling `observe` from both `pump_once` and `run`'s paused-discard branch.

## PauseClock::new

```rust
pub fn new() -> Self
```

Returns a fresh `PauseClock` with zero accumulated paused time.

## PauseClock::observe

```rust
pub fn observe(&mut self, now: u64, paused: bool) -> Option<u64>
```

Observe one frame-arrival tick at time `now` (ms).

### Inputs

- `now: u64` - the current wall-clock time, read at frame arrival (from `Clock::now_ms()` in the GPU path, or the frame's own already-clock-derived `ts.0` in the legacy path). *Why not always a fresh clock read:* the legacy path's `Frame::ts` is already the wall-clock time the frame was captured, so reusing it avoids passing an extra `Clock` through `RecordingSession::run`.
- `paused: bool` - whether capture is currently paused for this tick.

### Returns

- While `paused`: always `None` (the frame should be dropped). If there is a previous paused sample (`last_paused_at`), the elapsed time since it is added to `paused_ms` - this is how the accumulator measures real paused duration from a stream of ticks rather than a single start/end pair. A lone paused sample (no predecessor yet) contributes no duration by itself. `last_paused_at` is then set to `now`.
- While not `paused`: `Some(now - paused_ms)`, the timestamp to record (saturating subtraction). `last_paused_at` is reset to `None` so the next pause episode starts its own delta chain.

### Implementation

1. If `paused`: add `now - last_paused_at` to `paused_ms` when `last_paused_at` is `Some`; set `last_paused_at = Some(now)`; return `None`.
2. If not `paused`: set `last_paused_at = None`; return `Some(now.saturating_sub(paused_ms))`.

### Behaviors

- `unpaused_tick_before_any_pause_is_unadjusted`: `observe(1000, false)` on a fresh clock returns `Some(1000)`.
- `paused_ticks_return_none_and_drop_the_frame`: two consecutive paused calls both return `None`.
- `push_after_pause_excludes_the_paused_span`: push@1000, paused 1000..3000 (observed at both ends), push@3100 -> `Some(1100)` - the 2000ms pause is subtracted. This is the required scenario from the task brief.
- `accumulates_across_multiple_pause_episodes`: two separate pause episodes (200ms then 150ms) both contribute to `paused_ms`, giving a final adjusted ts of `700 - 350 = 350`.
- `single_paused_sample_contributes_no_duration_yet`: a single paused tick with no predecessor adds nothing to `paused_ms`; the very next unpaused tick is unadjusted.
