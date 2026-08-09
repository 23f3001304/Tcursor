# src-tauri/src/session/pacing.rs

Constant-frame-rate (CFR) capture helpers factored out of `RecordingSession` so the recording loop stays small. All state is passed by mutable reference, making the logic independently testable. Runs entirely on the caller's thread with no internal synchronization - the `AtomicBool` flags for stop and pause are owned by `recorder.rs` and shared across threads via `Arc`.

## frames_due

```rust
pub fn frames_due(active: u64, fps: u32) -> u64
```

Returns the total number of frame indices that should exist after `active` milliseconds of (unpaused) recording at `fps`. Frame 0 exists at active 0, so the result is `active * fps / 1000 + 1`.

### Inputs

- `active: u64` - elapsed recording time in ms, excluding paused stretches. *Why:* the caller computes this by subtracting `paused_ms` from wall time, so frame count is always anchored to real content time.
- `fps: u32` - target frame rate. *Why:* determines how many slots exist per second; cast to `u64` before multiplication to avoid overflow on long recordings.

### Returns

A `u64` frame count. At `active = 0, fps = 60` the result is 1 (frame 0 is always due immediately). At `active = 1000, fps = 60` the result is 61 (indices 0 through 60).

### Behaviors

- `frames_due_counts_from_first_and_excludes_paused`: verifies `frames_due(0, 60) == 1`, `frames_due(50, 60) == 4`, and `frames_due(1000, 60) == 61`.

## emit_due

```rust
pub fn emit_due(
    source: &mut dyn FrameSource, sink: &mut dyn FrameSink, frames: &mut u64, frame_ts: &mut Vec<u64>,
    start: u64, active: u64, fps: u32, emitted: u64, latest: &mut Frame,
) -> u64
```

Emits every not-yet-emitted frame index that is due by `active` time. Each emitted frame carries the most recent source frame (held and duplicated as needed) and a uniform timestamp `start + k * 1000 / fps`. Counts only successful sink pushes.

### Inputs

- `source: &mut dyn FrameSource` - the live capture source. *Why:* `drain_latest` is called per loop tick to refresh the held frame; the source may produce nothing (no new capture yet), in which case the previous `latest` is duplicated.
- `sink: &mut dyn FrameSink` - the video encoder. *Why:* receives the constructed frame; push errors are logged to stderr but do not abort the loop, so one bad frame does not stop the recording. A push that returns `Ok(false)` (the sink deliberately skipped the frame, e.g. a dimension mismatch) is likewise non-fatal but is not counted - see `frames`/`frame_ts` below.
- `frames: &mut u64` - running count of successfully encoded frames. *Why:* mutated here so `RecordingSession::frames_written` reflects the true count across both run modes.
- `frame_ts: &mut Vec<u64>` - per-frame capture timestamps in encode order. *Why:* written to `sync.json` on stop so the exporter can reconstruct the real timeline fps-agnostically.
- `start: u64` - clock value (ms) at recording start. *Why:* the base for uniform timestamp computation; all frame timestamps in `sync.json` are relative to this origin.
- `active: u64` - unpaused elapsed time in ms. *Why:* determines `frames_due`, capping how far ahead we emit.
- `fps: u32` - target frame rate. *Why:* sets the spacing between uniform timestamps (`1000 / fps` ms per frame).
- `emitted: u64` - frames already emitted in prior calls. *Why:* the function starts its counter at `emitted` and only emits indices `[emitted, due)`, preventing re-emission on the next tick.
- `latest: &mut Frame` - holds the most recently captured frame across calls. *Why:* BGRA data is cloned out so a gap in source output (no new capture) safely duplicates the last real frame.

### Returns

The new emitted count `k` after this call, suitable as the `emitted` argument on the next tick.

### Implementation

1. Compute `due = frames_due(active, fps)`.
2. Walk indices `k` from `emitted` to `due - 1`.
3. Per index: attempt `source.drain_latest()` and update `latest` if a new frame arrived. *Why drain_latest, not next_frame:* CFR mode does not block waiting for a frame; it takes whatever is available and duplicates otherwise.
4. Construct a new `Frame` with `latest`'s pixel data and a computed uniform timestamp `start + k * 1000 / fps as u64`.
5. Push to `sink`. On `Ok(true)`, increment `*frames` and append the timestamp to `frame_ts`. On `Ok(false)` (skipped by the sink, e.g. a dimension mismatch), do nothing - `k` still advances so the loop does not retry the same due index forever, but the skipped frame is not counted or timestamped. On `Err`, log to stderr and continue. *Why continue:* a transient encoder error should not stop the entire recording.
6. Return `k` (the final value of the counter).

### Behaviors

- `emit_due_emits_uniform_timestamps_and_duplicates`: verifies that after two calls covering `active = 0` then `active = 50ms` at 60fps, the recorded timestamps are `[1000, 1016, 1033, 1050]` with `frames == 4`, confirming uniform spacing and frame duplication when the source is exhausted.
- `emit_due_does_not_count_or_timestamp_skipped_frames`: with a sink whose `push` always returns `Ok(false)`, a single call covering `active = 50ms` at 60fps still advances the emitted counter to `4` (the due indices are still walked) but `frames` stays `0` and `frame_ts` stays empty.

## run_paced

```rust
pub fn run_paced(
    source: &mut dyn FrameSource, sink: &mut dyn FrameSink, frames: &mut u64, frame_ts: &mut Vec<u64>,
    stop: &AtomicBool, paused: &AtomicBool, clock: &dyn Clock, fps: u32,
)
```

The main CFR capture loop. Spins at ~2ms resolution, emitting one frame per `1/fps` of real (unpaused) time, each using the latest available source frame. Terminates when `stop` is set. Paused intervals are excluded from `active` time so the recorded timeline contains no gaps.

### Inputs

- `source: &mut dyn FrameSource` - live screen capture source. *Why:* frame data is polled via `drain_latest`; the loop blocks on the first real frame before recording begins.
- `sink: &mut dyn FrameSink` - video encoder. *Why:* receives constructed CFR frames via `emit_due`.
- `frames: &mut u64` - cumulative successful push count. *Why:* passed by reference so `RecordingSession` accumulates the total across delegation to this function.
- `frame_ts: &mut Vec<u64>` - per-frame timestamps. *Why:* same delegation pattern; the exporter reads this via `sync.json`.
- `stop: &AtomicBool` - shutdown signal from `recorder.rs`. *Why:* read with `SeqCst` ordering on every iteration so the thread stops promptly when `stop_recording` fires.
- `paused: &AtomicBool` - pause signal. *Why:* read with `SeqCst` on every iteration; while set, `paused_ms` accumulates so `active` shrinks and no new frames are emitted.
- `clock: &dyn Clock` - injectable wall clock. *Why:* testable without real time passing; `SystemClock` is used in production.
- `fps: u32` - target frame rate passed through to `emit_due` and `frames_due`.

### Returns

`()`. All output is through the mutable references.

### Implementation

1. Spin-wait (2ms sleep) for the first real frame from `source.drain_latest()`. *Why:* there is nothing to hold/duplicate until at least one real frame has arrived; returning early before that would emit black frames.
2. Record `start = clock.now_ms()` and initialize `emitted = 0`, `paused_ms = 0`, `pause_at: Option<u64> = None`.
3. Loop while `!stop`:
   a. If `paused`: record `pause_at` on the first paused tick, drain source to keep `latest` current, sleep 2ms, continue. *Why drain while paused:* the source buffer should not fill up during a pause; discarding is better than blocking.
   b. On resume: accumulate `paused_ms += now - pause_at`. *Why Option takeout:* `pause_at` is cleared so the next pause starts a fresh interval.
   c. Compute `active = now - start - paused_ms` (both subtractions use `saturating_sub` to stay non-negative).
   d. Call `emit_due` to push any frames due at this `active` time; advance `emitted`.
   e. Sleep 2ms. *Why 2ms:* coarse enough to avoid busy-spinning (cheap on battery), fine enough that at 60fps (16.67ms/frame) each frame is never more than ~2ms late.
