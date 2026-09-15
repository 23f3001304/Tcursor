# src-tauri/src/ports/capture.rs

Putting screen pixels in a file: which targets exist, where they sit, and the live pipeline that writes `video.mp4` and hands back the per-frame capture timestamps `sync.json` is built from. Pure traits and plain data - no OS type appears here.

## TargetId

```rust
pub enum TargetId { Primary, Display(usize), Window(u64) }
```

What a take records.

*Why typed rather than a `String`.* Four places parse `"window:0x…"` / `"display:N"` by hand today (`commands.rs`, `session/record/target_bounds.rs`, `session/record/gpu_record.rs`, `capture/windows_capture.rs`) and they once disagreed: `display:N` resolved through a different monitor ordering than the capture used, so the take took another monitor's origin and every cursor point was drawn shifted by the delta. Parsing once, at the boundary, is what stops that class of bug rather than fixing this instance of it.

`Primary` is a variant rather than `Display(0)` because the two are not the same call: the free functions reach the primary display through `Monitor::primary()` and `display:0` through `Monitor::from_index(0)`. They normally name the same screen and are not guaranteed to.

## TargetKind

```rust
pub enum TargetKind { Display, Window }
```

Which of the two kinds of thing a target is, for a picker that groups them. Today's `DisplayInfo::kind` carries the same distinction as the strings `"display"` and `"window"`.

## CaptureTarget

```rust
pub struct CaptureTarget { pub id: TargetId, pub label: String, pub kind: TargetKind }
```

One offerable capture target: its id, the label the Sources sheet shows, and its kind. The label stays a formatted string because it is presentation, and the HUD's display map already parses the size back out of it.

## CaptureGeometry

```rust
pub struct CaptureGeometry { pub w: u32, pub h: u32, pub origin_x: i32, pub origin_y: i32 }
```

Where a target's pixels are: the captured size, plus the desktop origin the mouse hook's coordinates are relative to. One type for both halves so a caller cannot take the size from one resolution of a target and the origin from another - which is exactly the shape the `display:N` origin bug had.

## FirstFrameSize

```rust
pub type FirstFrameSize = Option<Box<dyn FnOnce(u32, u32) + Send>>;
```

Told the replacement capture's FIRST frame size, once.

*Why it is part of the port and not an implementation detail.* A window's rectangle is not its capture: a maximized window reads 1944x1104 while WGC delivers 1920x1080, and `DWMWA_EXTENDED_FRAME_BOUNDS` still leaves a 14 px discrepancy on some frames. The switch record is written before the restart (the replacement's first frame can arrive while `switch` is still returning), so the only way it can hold the true size is for the capture to call back with it. The render crops the fitted picture by that size to the pixel; without the hook it crops by the window rect and leaves a black edge either side.

## CaptureRequest

```rust
pub struct CaptureRequest {
    pub target: TargetId,
    pub output: PathBuf,
    pub fps: u32,
    pub with_cursor: bool,
    pub prefer_compatibility: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
}
```

Everything a capture needs beside the port itself.

- `prefer_compatibility: bool` - the HUD's compatibility toggle. A hint, not an instruction: an adapter with one pipeline ignores it, and the HUD greys the toggle where it cannot be honoured.
- `stop` - the TAKE's stop flag, shared with the audio threads. See the decision below.
- `paused` / `totals` - the pause state, SHARED rather than driven through the port. The pause instant is stamped under the recorder's lock, from the recorder's clock, beside the flag flip, and every input stream and both capture paths subtract that one ledger. A `pause()` method on the port would move the stamp off that lock and let the streams disagree about a pause, which is why there is no such method.
- `ended: Notify` - called when the OS, not the user, ends the capture, so the app can run its normal Stop instead of leaving the HUD counting against a video that already stopped. An `Arc<dyn Fn>` keeps Tauri types out of the pipeline.

*The stop-flag decision (Batch D, 2026-09-15).* Section 4.1 of the architecture had no `stop` field; Batch A recorded that the adapter could not forward without one and left the choice open - either the request carries the take's flag, or the sink owns a private one and the audio threads get their own. **The request carries it.** Three reasons, in order of weight:

1. *It is the only one with literally zero behaviour change.* `stop` is what the LEGACY ffmpeg path's session loop polls (`platform::windows::capture::legacy::start_ffmpeg`), so today `recorder_stop` setting it ends that capture BEFORE the audio threads are joined and the input tracks written. A sink-private flag is only set inside `VideoSink::stop`, several hundred milliseconds later, so the compatibility path would have grown a tail of frames it does not have today. On the GPU path there is no difference at all - it never reads the flag - but "zero behaviour change on Windows" is the batch's first rule and one pipeline is not an exception to it.
2. *`paused` is already here, and `stop` is its twin.* Both are the take's shared state, stamped in one place and read by every stream; the port takes `paused` for exactly the reason it takes `stop`. Splitting them would leave the take with one shared pause edge and two stop edges, which is how streams start disagreeing about when a take ended.
3. *`VideoSink::stop` is a different thing from the flag.* `stop` means "the user asked to end the take, now"; `VideoSink::stop` means "finalize and hand me the result", and it blocks on flushing an encoder. The recorder needs the first without paying for the second, which is why the flag is separate from the method rather than redundant with it.

What it costs: `switch` also takes a `CaptureRequest`, and a stop flag makes no sense mid-take. In practice the field carries no ambiguity - `switch_display` passes `Running.stop`, the same `Arc` the sink already holds - and an adapter that cannot use it ignores it, exactly as it may ignore `prefer_compatibility`. A macOS adapter that can tell `stopCapture` from `didStopWithError` by other means does not have to read this at all.

The `ended(CAPTURE_CLOSED)` detection the flag exists for is unchanged: the legacy video thread still asks "did `run` return with `stop` still false?" and only then reports that the OS ended the capture.

## VideoSink

```rust
pub trait VideoSink: Send
```

A live recording video pipeline, owned by exactly one caller and consumed on stop. `Box<dyn VideoSink>` rather than `Arc<dyn VideoSink>` on purpose: every stop here takes the object by value, and an `Arc` cannot express that without a runtime unwrap.

## VideoSink::switch

```rust
fn switch(&mut self, req: CaptureRequest, on_first_frame: FirstFrameSize) -> Result<(), String>
```

Moves a running capture to another target while keeping the encoder, so the file stays one stream at one size and the editor never learns a second screen existed. Takes a full `CaptureRequest` rather than just a target, because the replacement capture needs the clock, the pause ledger and the ended-notify exactly as the first one did.

`Err` leaves the take without a capture. That is a salvageable state, not a lost take: the caller drops the switch record it wrote and the frames already encoded are still finalized.

## VideoSink::stop

```rust
fn stop(self: Box<Self>) -> VideoStopped
```

Stops the pipeline and finalizes the file. Never returns `Err`.

*Why not a `Result`.* A finalize failure used to propagate before `sync.json`, the `.tcursor` manifest and the recents entry were written, so a disk-full tail on a long take left a folder the app could not even open. The failure rides inside `VideoStopped::error` instead, alongside the frame timestamps, and the caller writes the session files from those first and surfaces the error after. A `Result` on the port puts that bug back.

## VideoSink::supports_switch

```rust
fn supports_switch(&self) -> bool
```

Whether `switch` can work at all on this pipeline, asked before the Sources sheet offers it rather than discovered by a failed call. On Windows the legacy ffmpeg pipeline cannot switch (its rawvideo pipe is sized once at start); on macOS `AVAssetWriter` has no obvious equivalent of the encoder handoff, so that adapter is expected to answer `false` at first.

## CapturePort

```rust
pub trait CapturePort: Send + Sync
```

The platform's screen capture: what can be recorded, where it is, and how to start it.

## CapturePort::list_targets

```rust
fn list_targets(&self) -> Vec<CaptureTarget>
```

Every display and application window the platform will offer as a capture target.

## CapturePort::bounds

```rust
fn bounds(&self, target: &TargetId) -> CaptureGeometry
```

The target's rectangle. Lives on the capture port, not the system port, so starting a capture and asking for its rectangle resolve the same target the same way; they once did not, and the take took another monitor's origin. Infallible by design - the Windows implementation falls back to the primary display and finally to 1920x1080 at the origin, so a caller always has a usable rectangle.

## CapturePort::start

```rust
fn start(&self, req: CaptureRequest) -> Result<(Box<dyn VideoSink>, CaptureGeometry), String>
```

Starts recording and hands back the live sink plus the geometry the take's one `ScreenInfo` is built from.

*Why there is no `dimensions()` on the sink.* It would be ambiguous: the size returned here is the target's estimate, while the file's real size is set by the first frame inside the capture handler. The estimate comes back once, at start; the truth arrives later, through `FirstFrameSize`.

`Result<_, String>` rather than `anyhow`, because this value flows straight to a Tauri command; adapters use `anyhow` internally and convert at this seam.

## TargetId::fmt

```rust
fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
```

The string form: `"display:N"`, `"window:0x<lowercase hex>"` (the `{:x}` shape `commands.rs` produces), and `"primary"`.

*Why `Primary` gets a string at all.* No parse site recognizes `"primary"`, and every one of them falls back to the primary display for anything it cannot parse - so the string is the same answer, spelled. That matters for `VideoSink::switch`, whose Windows signature takes a `&str` and has no way to express `None`.

## TargetId::from_str

```rust
fn from_str(s: &str) -> Result<Self, Self::Err>
```

Reads a target id. `type Err = Infallible`: none of the hand-written parse sites can fail either, because an id that is neither `display:N` nor `window:0x…` falls through to the primary display. This reproduces that exactly rather than inventing an error the recorder has no path for.

## TargetId::as_arg

```rust
pub fn as_arg(&self) -> Option<String>
```

The `Option<&str>` today's free functions take, where `None` IS the primary display. Use this, not `to_string`, when forwarding to `start_video`, `get_target_bounds` or `start_capture`, so the primary case stays the exact argument they already receive.

### Used by

- `src-tauri/src/platform/windows/capture.rs` - in `bounds` and `start`, to call `get_target_bounds` and `start_video`.

## TargetId::from_arg

```rust
pub fn from_arg(arg: Option<&str>) -> Self
```

The inverse: reads one of today's `Option<&str>` target arguments, or an id string from `list_displays`.

### Behaviors

Pinned in `capture_tests.rs` against the exact forms the parse sites accept: `display:0` / `display:2` round trip, `window:0x1` and `window:0x1a2b3c` round trip as lowercase unpadded hex, `Primary` is `None` as an argument and `"primary"` as a string, and everything unparseable (`""`, `display:x`, `window:0xzz`, `window:1`, `nonsense`) lands on `Primary` the way every parse site does.
