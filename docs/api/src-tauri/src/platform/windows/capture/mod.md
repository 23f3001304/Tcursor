# src-tauri/src/platform/windows/capture/mod.rs

The Windows capture adapter's root: the two pipeline choices (`VideoSink`, `start_video`) and the `ports::capture` implementations over them (`Win32Capture`, `Win32Sink`). Batch C1 moved the bodies here out of `session/record/`, so these are real implementations, not forwarders; the only thing left behind in the portable tree is `VideoStopped`, the value the port returns.

Batch D removed the compatibility re-exports and with them every caller outside this folder. `VideoSink`, `VideoStart` and `start_video` are now reached only through `Win32Capture` and `Win32Sink`, and `start_video`, `VideoSink::switch` and `VideoSink::stop_and_collect` are private to this module - the trait methods are the way in.

The pipeline picks GPU-native Media Foundation by default (cures game-capture lag - no per-frame readback), falling back to the legacy ffmpeg rawvideo pipe (`legacy/`) when the user forces it (the compatibility toggle) or the GPU encoder will not initialize. Both write `video.mp4` and return the per-frame capture timestamps (ms) the export needs for `sync.json`.

## VideoSink

```rust
pub enum VideoSink { Gpu(GpuRecorder), Ffmpeg { thread, halt, stopper }, Dead(FrameTimes) }
```

A live recording video pipeline. `Gpu` is the GPU-native path (`gpu::record::GpuRecorder`); `Ffmpeg` is the legacy WGC-readback -> `RecordingSession` -> `VfrSegments` path running on its own thread, kept as the manual + automatic fallback.

`Dead` is neither: it is where a take lands when `switch` stopped the capture and its replacement never started. Nothing is recording and `video.mp4` has been finalized, but the variant still holds the shared `FrameTimes` the dead capture was pushing into - the same `Arc`, not a snapshot, so the last frames it wrote on its way out are in it too. `stop_and_collect` turns that into a `VideoStopped` with an error and a full timestamp list, so a failed switch costs the take its tail and nothing more. Without it, `switch` would have nothing to leave in `&mut self` after moving the recorder into `GpuRecorder::restart`, and a failed restart would take `sync.json` with it.

The enum is NOT the platform seam. It encodes a pipeline choice plus one failure state; the seam is `ports::capture::VideoSink`, which `Win32Sink` implements over this.

## NO_GPU

```rust
const NO_GPU: &str = "switching needs the GPU encoder; turn the compatibility encoder off";
```

What `switch` refuses with when the take is on the legacy pipeline. Its rawvideo pipe is sized once at start and `RecordingSession::pump_once` ends the take on the first mismatched frame (`DISPLAY_CHANGED`), so there is nothing to restart into - and the message names the setting the user has to change rather than the internals.

## VideoStart

```rust
pub struct VideoStart {
    pub legacy: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}
```

Everything both capture paths need beside the target and the output path.

- `legacy: bool` - force the ffmpeg pipeline (the HUD's compatibility toggle). `CaptureRequest::prefer_compatibility` at the port.
- `stop: Arc<AtomicBool>` - the take's shared "we are stopping" flag, `CaptureRequest::stop` at the port. Only the legacy path reads it, to tell a user Stop apart from the OS ending the capture on its own.
- `totals: Arc<PauseTotals>` - the exact-span pause ledger both the video timestamps and every input stream subtract, so they cannot disagree about a pause.
- `ended: Notify` - called if the OS ends the capture on its own, OR the capture's own dimensions change mid-record, so the app can run its normal Stop instead of leaving the HUD counting against a video that already stopped.

## start_video

```rust
fn start_video(cfg: VideoStart, target_id: Option<&str>, video_path: &str) -> Result<(VideoSink, u32, u32, i32, i32), String>
```

Starts the video pipeline writing `video_path`. GPU-native unless `cfg.legacy` is set; on a `GpuRecorder::start` error it logs and falls back to ffmpeg, so recording never simply fails. Returns the sink plus the captured `(w, h, origin_x, origin_y)`, the origin read off the `CaptureGeometry` `target::get_target_bounds` returns.

## VideoSink::switch

```rust
fn switch(&mut self, cfg: VideoStart, target_id: &str, on_size: SizeHook) -> Result<(), String>
```

Moves a running capture to another display or window mid-take, keeping the encoder - so `video.mp4` stays one stream at one size and the editor never learns a second screen existed. `on_size` (`gpu::frames::SizeHook`) is told the replacement capture's first frame size; `switch_display` writes it into the switch record. The target's rectangle is not this function's business: `switch_display` asks `target::get_target_bounds` itself, before the restart, for the record and the `Remap`. `Err(NO_GPU)` on the legacy pipeline; `Err("display switch: ...")` when the restart itself failed, with the sink left as `Dead`.

### Implementation

1. Refuse anything but `Gpu`.
2. Take `frame_times()` off the live recorder and build the `Dead` stand-in from it, BEFORE anything is stopped.
3. `std::mem::replace(self, dead)` to move the recorder out - `GpuRecorder::restart` consumes it - then `restart(gpu, Some(target_id), on_size)`, whose returned `(w, h)` estimate is dropped: the true size reaches the record through `on_size`. On success the new recorder goes back into `self`; on failure `self` stays `Dead` and the error is returned with the `display switch:` prefix.

## VideoSink::stop_and_collect

```rust
pub fn stop_and_collect(self) -> VideoStopped
```

Stops the pipeline and finalizes `video.mp4`. GPU: delegates to `GpuRecorder::stop`. Ffmpeg: sets the halt flag, runs the stopper (WM_QUIT unblocks the WGC thread, closing the frame channel so `run()` ends), then joins the thread - a panicked thread becomes a `VideoStopped` with an `error` and no frames, rather than being propagated as a `Result`, so the caller's salvage path is identical in every failure mode. `Dead`: reads the timestamps a failed `switch` left behind and reports them with an error saying so; there is nothing left to stop or finalize.

## Win32Capture

```rust
pub struct Win32Capture;
```

`ports::capture::CapturePort` over Windows Graphics Capture. A unit struct because the platform holds no state of its own - the live state is all in the sink.

## Win32Sink

```rust
pub struct Win32Sink { sink: VideoSink, stop: Arc<AtomicBool> }
```

A live capture behind `ports::capture::VideoSink`, plus the stop flag its legacy pipeline reads.

*Why the flag is held rather than just passed on.* The legacy path's thread checks it to tell a user Stop apart from the OS ending the capture on its own: if `run` returns while the flag is still false, the capture ended by itself and the HUD is told. The sink keeps a clone so `Win32Sink::stop` can set it before collecting, whatever the caller did - the port's contract is "stop the pipeline", not "set this flag first". Batch D settled where the `Arc` comes from: `CaptureRequest::stop`, the take's one flag, also read by the system-audio thread. The reasoning is in `ports/capture.md`.

## video_start

```rust
fn video_start(req: &CaptureRequest) -> VideoStart
```

A `VideoStart` from a port request: everything but the target and the output path. `prefer_compatibility` is `legacy`, and the rest are field-for-field clones of the shared `Arc`s, so the capture keeps subtracting the same pause ledger every input stream does.

## Win32Sink::switch

```rust
fn switch(&mut self, req: CaptureRequest, on_first_frame: FirstFrameSize) -> Result<(), String>
```

Forwards to `VideoSink::switch`. Two conversions:

- The target goes in as a string, because that signature takes `&str` and not `Option<&str>`. `TargetId::Primary` therefore goes in as `"primary"`, which `TargetId::from_arg` reads back as `Primary` at the one place that parses it (`gpu::record::start_capture`), landing on the same `Monitor::primary()` the `None` form does.
- `FirstFrameSize` and `gpu::frames::SizeHook` are the same type (`Option<Box<dyn FnOnce(u32, u32) + Send>>`), so the hook passes straight through and the switch record is still corrected to the replacement capture's real first frame.

## Win32Sink::stop

```rust
fn stop(self: Box<Self>) -> VideoStopped
```

Sets the stop flag, then forwards to `VideoSink::stop_and_collect`, which is the order `recorder_stop` uses. Returns `VideoStopped` unchanged, error field and all, so a finalize failure still reaches the caller alongside the frame timestamps rather than instead of them.

## Win32Sink::supports_switch

```rust
fn supports_switch(&self) -> bool
```

True only for the GPU pipeline. The legacy rawvideo pipe is sized once at start and its session ends the take on the first mismatched frame, and a `Dead` sink has nothing left to restart - which is exactly what `VideoSink::switch` refuses for, so answering from the variant matches the refusal it would have given.

## Win32Capture::list_targets

```rust
fn list_targets(&self) -> Vec<CaptureTarget>
```

`target::list_targets()` verbatim. The enumeration itself now produces `CaptureTarget`s; `commands::list_displays` is the one that converts DOWN to the frontend's `DisplayInfo`, so the port no longer re-parses a string the command just formatted.

## Win32Capture::bounds

```rust
fn bounds(&self, target: &TargetId) -> CaptureGeometry
```

`target::get_target_bounds(target.as_arg().as_deref())`, returned as it stands - that function already produces the `CaptureGeometry` this trait method promises. `as_arg` and not `to_string`, so the primary display goes in as the `None` that function already receives.

## Win32Capture::start

```rust
fn start(&self, req: CaptureRequest) -> Result<(Box<dyn VideoSinkPort>, CaptureGeometry), String>
```

Clones the request's stop flag, builds a `VideoStart` from the request, and forwards to `start_video` with the output path as a string. The returned `(w, h, origin_x, origin_y)` becomes the `CaptureGeometry` the take's one `ScreenInfo` is built from; the sink is boxed with the flag.

The error is passed through as the `String` `start_video` already produces - no `anyhow` conversion, because this value flows to a Tauri command.
