# src-tauri/src/session/recording_session.rs

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Tracks how many frames were successfully encoded and records their timestamps. Deliberately has no threading primitives of its own - it is always driven from inside the `"video"` thread in `recorder.rs`, which passes in the shared stop/pause flags by reference.

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

- `src-tauri/src/session/recording_session.rs` - read by `state()`, written by `stop_and_finalize`.

## RecordingSession

```rust
pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
}
```

Owns the capture source and encoder sink for one recording.

- `source: Box<dyn FrameSource>` - abstracted frame producer (WGC capture in production, `FakeFrameSource` in tests). *Why Box<dyn>:* allows injecting test doubles without conditional compilation.
- `sink: Box<dyn FrameSink>` - abstracted encoder (ffmpeg in production, `FakeFrameSink` / `FailingSink` in tests).
- `state: SessionState` - tracks whether the session is active or stopped. *Why state field:* `stop_and_finalize` consumes `self` (by value), so reading `state` before that call is the only way to inspect whether finalization has occurred via an earlier reference.
- `frames: u64` - count of frames for which `sink.push` returned `Ok`. *Why count only successes:* encoder errors are non-fatal; the count must reflect what is actually in the file, not what was attempted.
- `frame_ts: Vec<u64>` - millisecond capture timestamp of each successfully encoded frame. *Why Vec:* unbounded; a 1-hour recording at 60fps produces 216000 entries (~1.7 MB), acceptable in memory.

### Used by

- `src-tauri/src/session/recorder.rs` - constructed and driven inside the `"video"` thread; `frame_timestamps` and `stop_and_finalize` called after the run loop.
- `src-tauri/src/session/pacing.rs` - receives `source`, `sink`, `frames`, and `frame_ts` as mutable references from `run_paced`.

## RecordingSession::new

```rust
pub fn new(source: Box<dyn FrameSource>, sink: Box<dyn FrameSink>) -> Self
```

Constructs a `RecordingSession` in the `Recording` state.

### Inputs

- `source: Box<dyn FrameSource>` - the frame producer. *Why taken by value:* `RecordingSession` owns the source for its lifetime; no shared access is needed.
- `sink: Box<dyn FrameSink>` - the encoder. *Why taken by value:* `stop_and_finalize` consumes `self` to call `sink.finish()`, which requires ownership.

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

Returns a slice of millisecond capture timestamps, one per successfully encoded frame, in encode order. Used by `recorder.rs` to populate `SyncLog::frames`.

## RecordingSession::pump_once

```rust
pub fn pump_once(&mut self) -> bool
```

Pulls one frame from `source` and pushes it to `sink`. Returns `false` when the source is exhausted.

### Inputs

`&mut self` - mutates `frames` and `frame_ts` on success.

### Returns

`true` if the source returned a frame (even if the sink push failed). `false` when `source.next_frame()` returns `None`, signalling end of source.

### Implementation

1. Call `source.next_frame()`. On `None`, return `false`.
2. Call `sink.push(&frame)`. On `Ok`, increment `frames` and push `frame.ts.0` onto `frame_ts`. On `Err`, log to stderr. *Why log but continue:* a single encoder hiccup should not abort a potentially long recording.
3. Return `true` (the source had a frame, regardless of sink outcome).

### Behaviors

- `pumps_all_frames_then_reports_exhausted`: three pumps succeed; the fourth returns `false`; `frames_written` is 3.
- `frames_written_counts_only_successful_pushes`: with `FailingSink`, two frames are returned by the source (pump returns `true`) but `frames_written` stays 0.

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

Like `run_until_stopped`, but while `paused` is set frames are pulled and discarded instead of encoded, so paused time is excluded from the recording.

### Inputs

- `stop: &AtomicBool` - termination flag, SeqCst load.
- `paused: &AtomicBool` - pause flag, SeqCst load. *Why checked per-iteration:* allows toggling mid-loop; any latency is bounded by the time for one `next_frame` call.

### Implementation

1. Loop while `!stop.load(SeqCst)`.
2. If `paused`: call `source.next_frame()` and discard. Break if `None`. *Why discard rather than skip the call:* on a variable-FPS source, not reading frames would let the internal buffer fill indefinitely.
3. If not paused: call `pump_once`; break on `false`.

### Behaviors

- `run_encodes_when_not_paused`: two frames, both encoded; `frames_written == 2`.
- `run_discards_frames_while_paused`: two frames available, all discarded; `frames_written == 0`.

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
2. Call `self.sink.finish()?`. *Why `?` here:* the caller (`recorder.rs`) propagates the error to the frontend via `stop_recording`'s `Result`.
3. Return `Ok(self.frames)`.

### Behaviors

- `finalize_returns_count_and_marks_stopped`: after one successful pump, `stop_and_finalize` returns `Ok(1)`.
