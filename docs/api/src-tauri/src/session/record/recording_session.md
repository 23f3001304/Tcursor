# src-tauri/src/session/record/recording_session.rs

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Tracks how many frames were successfully encoded and records their timestamps. Deliberately has no threading primitives of its own - it is always driven from inside the `"video"` thread spawned by `video_sink::start_ffmpeg`, which passes in the shared stop/pause flags by reference.

## SessionState

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState { Idle, Recording, Stopped }
```

Lifecycle state of a `RecordingSession`.

- `Idle` - declared but never reached by the current constructor (`new` sets state to `Recording` immediately). *Why included:* reserved for future use if construction and start are split.
- `Recording` - active; frames may be pumped.
- `Stopped` - `stop_and_finalize` has been called; the sink has been flushed and dropped.

### Used by

- `src-tauri/src/session/record/recording_session.rs` - read by `state()`, written by `stop_and_finalize`.

## RecordingSession

```rust
pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
    pause_clock: PauseClock,
    mismatched: bool,
}
```

Owns the capture source and encoder sink for one recording.

- `source: Box<dyn FrameSource>` - abstracted frame producer (WGC capture in production, `FakeFrameSource` in tests). *Why Box<dyn>:* allows injecting test doubles without conditional compilation.
- `sink: Box<dyn FrameSink>` - abstracted encoder (ffmpeg in production, `FakeFrameSink` / `FailingSink` in tests).
- `state: SessionState` - tracks whether the session is active or stopped. *Why state field:* `stop_and_finalize` consumes `self` (by value), so reading `state` before that call is the only way to inspect whether finalization has occurred via an earlier reference.
- `frames: u64` - count of frames for which `sink.push` returned `Ok(true)`. *Why count only actual writes:* encoder errors are non-fatal, and a sink may also deliberately skip a frame (e.g. `FfmpegFrameSink`'s dimension-mismatch guard, returning `Ok(false)`) - the count must reflect what is actually in the file, not what was attempted.
- `frame_ts: Vec<u64>` - millisecond capture timestamp of each successfully encoded frame, already shifted to exclude paused time. *Why Vec:* unbounded; a 1-hour recording at 60fps produces 216000 entries (~1.7 MB), acceptable in memory.
- `pause_clock: PauseClock` - places each arriving frame on the recording clock, reading the shared `PauseTotals` ledger so every paused span is removed (see `pause_clock.rs`). *Why the session holds it rather than computing timestamps inline:* it also owns the first-encoded-frame origin and the no-advance guard, both of which must be per-session state.
- `mismatched: bool` - set the first time `sink.push` reports `Ok(false)` (Task 5, finding H1: a mid-record window resize/maximize or display resolution/rotation change). Read by `video_sink::start_ffmpeg` after `run` returns, to tell that apart from the source simply running dry (the OS closing the capture), which gets a different HUD reason (`DISPLAY_CHANGED` vs `CAPTURE_CLOSED`).

### Used by

- `src-tauri/src/session/record/video_sink.rs` - constructed and driven inside the `"video"` thread; `frames_written` / `frame_timestamps` are read BEFORE `stop_and_finalize`, so they survive a finalize failure.
- `src-tauri/src/session/pacing.rs` - receives `source`, `sink`, `frames`, and `frame_ts` as mutable references from `run_paced`.

## RecordingSession::new

```rust
pub fn new(source: Box<dyn FrameSource>, sink: Box<dyn FrameSink>, totals: Arc<PauseTotals>) -> Self
```

Constructs a `RecordingSession` in the `Recording` state.

### Inputs

- `source: Box<dyn FrameSource>` - the frame producer. *Why taken by value:* `RecordingSession` owns the source for its lifetime; no shared access is needed.
- `sink: Box<dyn FrameSink>` - the encoder. *Why taken by value:* `stop_and_finalize` consumes `self` to call `sink.finish()`, which requires ownership.
- `totals: Arc<PauseTotals>` - the recorder's exact-span pause ledger, the same `Arc` every input tracker holds; it becomes this session's `PauseClock`. *Why passed in rather than owned:* the ledger is stamped at the pause/resume toggles by the recorder commands, which is the only place the true span is known.

### Returns

A new `RecordingSession` with `frames = 0` and an empty `frame_ts`.

## RecordingSession::state

```rust
pub fn state(&self) -> SessionState
```

Returns the current `SessionState`. No locks required.

## RecordingSession::frames_written

```rust
pub fn frames_written(&self) -> u64
```

Returns the count of frames for which `sink.push` returned `Ok`.

## RecordingSession::frame_timestamps

```rust
pub fn frame_timestamps(&self) -> &[u64]
```

Returns a slice of millisecond capture timestamps (pause-compressed), one per successfully encoded frame, in encode order. Read by the `"video"` thread to populate `VideoStopped::frame_ts`, which becomes `SyncLog::frames`.

## RecordingSession::dimension_mismatch

```rust
pub fn dimension_mismatch(&self) -> bool
```

True once a dimension-mismatched frame has ended the take early (see `mismatched`). Read by `video_sink::start_ffmpeg` after `run` returns, to choose between the `DISPLAY_CHANGED` and `CAPTURE_CLOSED` early-end reasons.

## RecordingSession::pump_once

```rust
pub fn pump_once(&mut self) -> bool
```

Pulls one frame from `source` and pushes it to `sink`. Returns `false` when the source is exhausted OR the sink just reported the FIRST dimension mismatch.

### Inputs

`&mut self` - mutates `frames`, `frame_ts`, `pause_clock`, and (on the first mismatch) `mismatched`.

### Returns

`true` if the source returned a frame AND it was either encoded or dropped for an ordinary reason (a pause-clock decline, or a sink `Err`). `false` when `source.next_frame()` returns `None` (source exhausted), OR when `sink.push` reports the FIRST `Ok(false)` (a dimension mismatch - Task 5, finding H1) - the two are told apart by `dimension_mismatch()` afterward.

### Implementation

1. Call `source.next_frame()`. On `None`, return `false`.
2. Feed `frame.ts.0` through `self.pause_clock.tick(_, false)`. A `None` back means the clock declined the frame (the pause-compressed time would not advance), so nothing is pushed, counted or timestamped - the same treatment a mismatched frame gets.
3. Otherwise set `frame.ts = tick.sync_ms` and call `sink.push(&frame)`. *Why the frame's own timestamp is rebased and not just the recorded one:* `VfrSegments` builds its concat offsets out of each part's first RECORDING-clock timestamp, so a raw capture time there would re-introduce the paused span the whole C1 fix removes; every other sink ignores `ts`, and the rebase is a field write on an owned frame, so it costs nothing. On `Ok(true)`, increment `frames` and push `tick.sync_ms` onto `frame_ts`. On `Ok(false)` (a dimension mismatch - H1: window resize/maximize, display resolution/rotation change), set `mismatched = true` and return `false` immediately - do not count or timestamp this frame, and stop pumping so the prior span finalizes cleanly instead of the source running on unread (this used to fall through and keep pumping, discarding every later frame forever). On `Err`, log to stderr. *Why log but continue:* a single encoder hiccup should not abort a potentially long recording.
4. Return `true` (the source had a frame and nothing told the loop to stop).

### Behaviors

- `pumps_all_frames_then_reports_exhausted`: three pumps succeed; the fourth returns `false`; `frames_written` is 3.
- `frames_written_counts_only_successful_pushes`: with `FailingSink`, two frames are returned by the source (pump returns `true`) but `frames_written` stays 0 AND `frame_timestamps()` stays empty - a frame the sink errored on is not in the video, so it must not be in `sync.json` either (which, since M1's salvage, is written even when the take failed to finalize). Same invariant the GPU path's `record_if_encoded` pins for `send_frame`.
- `a_dimension_mismatch_stops_pumping_instead_of_skipping_forever` (`recording_session_dim_tests.rs`, Task 5, finding H1): with `SkippingSink` (`push` always returns `Ok(false)`), `pump_once` returns `false` on the very first call, `frames_written` stays 0, `frame_timestamps()` stays empty, and `dimension_mismatch()` is `true`.

## RecordingSession::run_until_stopped

```rust
pub fn run_until_stopped(&mut self, stop: &AtomicBool)
```

Loops calling `pump_once` until `stop` is set or the source is exhausted. No pause support - used for simple or test scenarios.

### Inputs

- `stop: &AtomicBool` - checked with `SeqCst` load before each pump. *Why SeqCst:* must observe the flag written by another thread with no reordering.

### Behaviors

- `run_until_stopped_halts_on_flag`: with `stop` pre-set to `true`, zero frames are encoded.

## RecordingSession::run

```rust
pub fn run(&mut self, stop: &AtomicBool, paused: &AtomicBool)
```

Like `run_until_stopped`, but while `paused` is set frames are pulled and discarded instead of encoded, and each pause -> resume boundary `split`s the sink.

### Inputs

- `stop: &AtomicBool` - termination flag, SeqCst load.
- `paused: &AtomicBool` - pause flag, SeqCst load. *Why checked per-iteration:* allows toggling mid-loop; any latency is bounded by the time for one `next_frame` call.

### Implementation

1. Loop while `!stop.load(SeqCst)`, tracking a local `was_paused`.
2. If `paused`: set `was_paused`, then call `source.next_frame()` and drop the frame; break on `None`. *Why pull rather than skip the call:* on a variable-FPS source, not reading frames would let the internal buffer fill indefinitely. *Why the discarded frames are no longer fed to `PauseClock`:* they used to BE the pause measurement, which made the removed span depend on whether the screen happened to be changing (finding H2); the span now comes from the `PauseTotals` ledger stamped at the toggles.
3. If not paused: if `was_paused` was set, clear it and call `sink.split()` (logging a failure, never aborting the take) - this is what keeps the paused span out of the wall-clock-stamped ffmpeg output's own PTS, see `encode::vfr_segments`. Then call `pump_once`; break on `false`.

### Behaviors

- `run_encodes_when_not_paused`: two frames, both encoded; `frames_written == 2`.
- `run_discards_frames_while_paused`: two frames available, all discarded; `frames_written == 0`.
- `run_shifts_post_pause_timestamps_by_the_ledger_span`: an unpaused push at t=1000, then the LEDGER paused 1000..3000 with no frame arriving at all, then an unpaused push at t=3100, yields `frame_timestamps() == [1000, 1100]` - the static-desktop case the old frame-delta accumulator scored as 0 ms. The same test asserts the SINK saw `[1000, 1100]` too, pinning the rebased `Frame::ts` contract `VfrSegments` depends on.
- `run_splits_the_sink_on_resume`: a source that clears the pause flag partway through one `run` call produces exactly one `split`, and only the post-resume frame is encoded.
- `run_never_splits_without_a_pause`: an unpaused run splits zero times, so a recording that is never paused still writes exactly one file.
- `run_ends_the_take_at_the_first_mismatch_not_every_frame_after` (`recording_session_dim_tests.rs`, Task 5, finding H1): with a source of 3 frames and a sink that always reports a mismatch, `run` calls `sink.push` exactly once - not three times - proving the loop really stops after the first mismatch instead of retrying every subsequently pulled frame (the old "skip forever" behavior).
- `a_mismatch_after_some_frames_still_finalizes_the_prior_span_cleanly` (`recording_session_dim_tests.rs`): a sink that writes the first 2 of 4 frames then reports every later one as mismatched leaves `frames_written == 2`, `frame_timestamps() == [0, 33]`, `dimension_mismatch() == true`, and a subsequent `stop_and_finalize()` still returns `Ok(2)` - the prior span survives intact.

## RecordingSession::run_paced

```rust
pub fn run_paced(&mut self, stop: &AtomicBool, paused: &AtomicBool, clock: &dyn Clock, fps: u32)
```

Constant-frame-rate (game-mode) capture. Delegates entirely to `super::pacing::run_paced`, passing mutable references to `source`, `sink`, `frames`, and `frame_ts`.

### Inputs

- `stop / paused: &AtomicBool` - same flags as `run`, forwarded to `pacing::run_paced`.
- `clock: &dyn Clock` - injectable wall clock for uniform timestamp generation.
- `fps: u32` - target frame rate determining emit cadence.

### Implementation

1. Call `pacing::run_paced(&mut *self.source, &mut *self.sink, &mut self.frames, &mut self.frame_ts, stop, paused, clock, fps)`. *Why delegate:* the CFR algorithm is non-trivial; factoring it into `pacing.rs` keeps both files under 200 lines and makes the pacing logic independently testable.

## RecordingSession::stop_and_finalize

```rust
pub fn stop_and_finalize(mut self) -> std::io::Result<u64>
```

Marks the session stopped, flushes the sink, and returns the total frame count.

### Inputs

`mut self` - consumes the session. *Why by value:* `FrameSink::finish` is defined as `fn finish(self: Box<Self>)`, requiring ownership of the boxed sink; consuming `RecordingSession` is the simplest way to hand that ownership over.

### Returns

`Ok(frames)` on successful flush. `Err(io::Error)` if `sink.finish()` fails (e.g. ffmpeg process exited with an error).

### Implementation

1. Set `self.state = SessionState::Stopped`.
2. Call `self.sink.finish()?`. *Why `?` here:* the caller (the `"video"` thread in `video_sink.rs`) turns the error into `VideoStopped::error`, which the stop path surfaces to the frontend AFTER the session files are written.
3. Return `Ok(self.frames)`.

### Behaviors

- `finalize_returns_count_and_marks_stopped`: after one successful pump, `stop_and_finalize` returns `Ok(1)`.
