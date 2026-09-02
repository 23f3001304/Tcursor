# src-tauri/src/session/record/gpu_frames.rs

The WGC frame callback for the GPU-native path: hardware-encode each arriving frame on the GPU (no readback) with a pause-aware PTS, and record its capture time for `sync.json`. Split from `gpu_record.rs` (which owns the encoder settings and the recorder lifecycle) so both stay under the 200-line cap.

## FrameTimes

```rust
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;
```

Capture times (ms, pause-compressed) of the frames actually encoded, in encode order - i.e. exactly `sync.json`'s `frames[]`. Shared between the capture thread (which pushes) and `GpuRecorder::stop` (which clones it out), so the timestamps outlive the encoder and survive a finalize failure.

## CapFlags

```rust
pub struct CapFlags {
    pub encoder: VideoEncoder,
    pub clock: Arc<dyn Clock>,
    pub frame_ts: FrameTimes,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub dims: (u32, u32),
}
```

Everything `Cap` needs, handed to it through `Settings`' flags slot (the crate constructs the handler itself, on its own thread, so this is the only way in).

- `totals: Arc<PauseTotals>` - the recorder's exact-span pause ledger, shared with every input tracker.
- `ended: Notify` - called from `on_closed` when the OS ends the capture, or from `on_frame_arrived` when the capture's own dimensions change mid-record.
- `dims: (u32, u32)` - the `(w, h)` `encoder` above was configured for (`gpu_record::GpuRecorder::start` threads the same values it built the encoder from), seeding `Cap`'s `DimGuard`.

## Cap

```rust
pub struct Cap {
    pub encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
    pause_clock: PauseClock,
    ended: Notify,
    gfx: Context<()>,
    scratch: Vec<u8>,
    dims: DimGuard,
}
```

The `GraphicsCaptureApiHandler` the crate runs on its capture thread.

- `encoder: Option<VideoEncoder>` - `pub` because `GpuRecorder::stop` reaches through the crate's `callback()` handle to `take` and finalize it. `None` after `stop`, `on_closed`, or a dimension-mismatch early end has taken it.
- `pause_clock: PauseClock` - the frame's place on the recording clock, from the shared ledger.
- `gfx: Context<()>` - the capture's D3D device + device context, kept only so `on_frame_arrived` can rebuild the incoming frame around a rebased timestamp. *Why a `Context<()>` and not two typed fields:* the `windows` crate that names `ID3D11Device`/`ID3D11DeviceContext` here is windows-capture's own (0.61), a different version from this crate's direct dependency (0.58), so those two types cannot be written in a field declaration - while `windows_capture::capture::Context<()>` can, and its `pub` fields carry the values with their types inferred.
- `scratch: Vec<u8>` - the readback buffer `Frame::new` requires. Never touched: the rebuilt frame only ever reaches `send_frame`, which reads its surface and its timestamp and nothing else.
- `dims: DimGuard` (`dim_guard.rs`) - latches the FIRST frame whose size no longer matches `CapFlags::dims` (finding H1: a maximize/resize, display resolution/rotation change, or dock/undock mid-record).

## Cap::new

```rust
fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error>
```

Destructures the crate's `Context` so the flags are consumed while the device and device context are re-boxed into `gfx`, and builds the `PauseClock` over the ledger from `CapFlags`.

## record_if_encoded

```rust
fn record_if_encoded(frame_ts: &FrameTimes, sync_ms: u64, encoded: anyhow::Result<()>) -> anyhow::Result<()>
```

Appends `sync_ms` to `frame_ts` only once the frame it describes is actually IN the file. `encoded` is the send's own result, evaluated by the caller *before* this runs, and its error propagates with `frame_ts` untouched.

The ordering IS the guarantee, and it is a named function so it can be tested without WGC or a real `VideoEncoder`. The push used to come first, which was free while a send error also meant `sync.json` was never written at all - M1's salvage ended that, since the session files are now written from whatever was recorded even when the finalize failed. A pre-push would therefore leave a trailing timestamp describing a frame the file does not contain: the frame<->timestamp desync this whole task exists to remove, reintroduced at the very end of a failing take. The legacy path pins the same invariant through `RecordingSession::pump_once`, which only records on `Ok(true)`.

### Behaviors

- `an_encoded_frame_gets_its_timestamp`: an `Ok` send appends `sync_ms`.
- `a_failed_send_leaves_the_timestamps_untouched`: after one successful frame, a failing send returns `Err` and `frame_ts` still holds only the first timestamp.
- `the_encode_error_is_propagated_verbatim`: the send's error reaches the handler unchanged, so `CaptureControl::stop` still propagates it and the stop path still reports the take as failed.

## Cap::on_frame_arrived

```rust
fn on_frame_arrived(&mut self, frame: &mut Frame, ctl: InternalCaptureControl) -> Result<(), Self::Error>
```

Encodes one frame, drops it, or - on the first dimension mismatch - ends the take.

### Implementation

1. Check `dims.mismatched((frame.width(), frame.height()))` FIRST, before any pause/encode work. On `true` (finding H1: a maximize/resize, display resolution/rotation change, or dock/undock changed the frame size mid-record): finalize the encoder exactly like `on_closed` does (`encoder.take().map_or(Ok(()), |e| e.finish())`), call `ended(DISPLAY_CHANGED)`, then `ctl.stop()` - the SAME internal-halt mechanism `GpuRecorder::stop`'s external `CaptureControl::stop` uses, which also guarantees no later frame (mismatched or not) ever reaches this handler again - and return the finalize result. `DimGuard` itself only ever reports `true` once, so this branch cannot run twice even without that guarantee.
2. Read `clock.now_ms()` and the `paused` flag, and ask `pause_clock.tick`. `None` (paused, or no advance) returns `Ok(())` with nothing recorded and nothing encoded.
3. If the encoder is already gone (`on_closed` finalized the MP4 because the OS ended the capture), return `Ok(())` - this frame is not going into the file, so it must not go into `sync.json` either.
4. Rebuild the frame with `Frame::new` around the SAME GPU surface and texture (no readback, no copy) but with `timestamp().Duration = tick.pts_100ns`, and `send_frame` that. *Why:* `send_frame` stamps the MF sample with `frame.timestamp() - first_timestamp`, i.e. the raw WGC `SystemRelativeTime` QPC capture instant. That keeps every paused span alive in `video.mp4`'s own PTS while `sync.json`, the WAVs and all the input streams drop it; the export then decodes that file 1:1 against `sync.json`'s clock and gets a pause-length frozen span, everything else running ahead of the picture from the resume on, and a truncated tail (finding C1). The crate exposes no way to override the timestamp on the zero-copy path, but `Frame::new`, `as_raw_surface` and `as_raw_texture` are all public, so the rebuild is ordinary API use.

5. Hand the send's result to `record_if_encoded`, which appends `tick.sync_ms` only if it succeeded.

## Cap::on_closed

```rust
fn on_closed(&mut self) -> Result<(), Self::Error>
```

Runs only when the OS closes the capture (the recorded window closed, the display was unplugged/disabled/slept), NOT on a WM_QUIT stop - `GpuRecorder::stop` finalizes that case. Finalizes the encoder so the MP4 is still valid, then calls `ended(CAPTURE_CLOSED)`.

*Why the notification matters (finding M2):* without it the video simply ends here and `GpuRecorder::stop` later returns the truncated timestamps as a perfectly normal success - so the HUD keeps counting, the mic keeps writing narration, and the whole tail of the session does not exist. The notification is sent even if `finish()` fails, and the finalize error is still returned afterwards.
