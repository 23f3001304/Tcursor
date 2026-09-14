# src-tauri/src/session/record/gpu_frames.rs

The WGC frame callback for the GPU-native path: hardware-encode each arriving frame on the GPU (no readback) with a pause-aware PTS, and record its capture time for `sync.json`. Split from `gpu_record.rs` (which owns the encoder settings and the recorder lifecycle) so both stay under the 200-line cap; the tests live in `gpu_frames_tests.rs` under `#[path]`, and the mid-take encoder handover in `gpu_restart.rs`, for the same reason.

## FrameTimes

```rust
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;
```

Capture times (ms, pause-compressed) of the frames actually encoded, in encode order - i.e. exactly `sync.json`'s `frames[]`. Shared between the capture thread (which pushes) and `GpuRecorder::stop` (which clones it out), so the timestamps outlive the encoder and survive a finalize failure.

## SizeHook

```rust
pub type SizeHook = Option<Box<dyn FnOnce(u32, u32) + Send>>;
```

Told the raw `(width, height)` of a capture's FIRST frame, once, by `Cap::on_frame_arrived`. `switch_display` hands one in through `VideoSink::switch` -> `GpuRecorder::restart` -> `CapFlags::on_size` to write the switched-to target's true pixel size into its `DisplaySwitch` (`SegmentLog::set_display_size`): the estimate from the target's window rect is off by the invisible DWM borders, and `export::render::spans` crops the fitted picture by this size to the pixel. A cold start passes `None`.

## EncoderSpec

```rust
#[derive(Clone)]
pub struct EncoderSpec {
    pub fps: u32,
    pub path: String,
}
```

How to build the encoder ONCE the real capture size is known. `GpuRecorder::start` cannot build it: `GetWindowRect` and the monitor dimensions are only an *estimate* of what WGC will deliver, so this carries the settings that do not depend on size and `Cap::on_frame_arrived` supplies the size from the first real frame. `Clone` so `Cap::take_seed` can hand it on to a replacement capture whose predecessor had not built an encoder yet.

## CapFlags

```rust
pub struct CapFlags {
    pub enc: EncoderSpec,
    pub clock: Arc<dyn Clock>,
    pub frame_ts: FrameTimes,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub(super) seed: Option<EncoderSeed>,
    pub(super) on_size: SizeHook,
}
```

Everything `Cap` needs, handed to it through `Settings`' flags slot (the crate constructs the handler itself, on its own thread, so this is the only way in).

- `totals: Arc<PauseTotals>` - the recorder's exact-span pause ledger, shared with every input tracker.
- `ended: Notify` - called from `on_closed` when the OS ends the capture.
- `seed: Option<EncoderSeed>` (`gpu_restart.rs`) - `Some` only for the replacement capture a mid-take display switch starts: the encoder, the size it was built for and the recording clock of the capture this one takes over from, so `video.mp4` keeps one encoder and one timeline across the switch. `pub(super)` because `EncoderSeed` is, and because nothing outside the `record` group ever builds a `CapFlags`.
- `on_size: SizeHook` - `Some` for the replacement capture of a display switch (see `SizeHook`), `None` for a cold start.

## Cap

```rust
pub struct Cap {
    pub encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
    pub(super) pause_clock: PauseClock,
    pub(super) totals: Arc<PauseTotals>,
    ended: Notify,
    pub(super) handed_over: bool,
    gfx: Context<()>,
    scratch: Vec<u8>,
    pub(super) enc: EncoderSpec,
    pub(super) enc_dims: (u32, u32),
    fit: FrameFit,
    on_size: SizeHook,
}
```

The `GraphicsCaptureApiHandler` the crate runs on its capture thread. The `pub(super)` fields are the ones `gpu_restart::Cap::take_seed` moves out when a display switch hands this capture's encode to its replacement.

- `encoder: Option<VideoEncoder>` - `None` until the first frame arrives and gives it a size (or until a `seed` supplies one already built), and `None` again after `stop`, `on_closed` or `take_seed` has taken it. `pub` because `GpuRecorder::stop` reaches through the crate's `callback()` handle to `take` and finalize it.
- `pause_clock: PauseClock` - the frame's place on the recording clock, from the shared ledger. Inherited whole from a `seed`, so the PTS base and the monotonic check continue across a switch instead of restarting at zero mid-file.
- `totals: Arc<PauseTotals>` - the ledger `pause_clock` reads, kept beside it only so `take_seed` can move the real clock out and leave a fresh one in its place: a handler must stay valid until its thread exits.
- `handed_over: bool` - set by `take_seed`. Read by `on_closed`, which must not report the take as ended once the encoder belongs to a capture that is still running on another display.
- `gfx: Context<()>` - the capture's D3D device + device context, kept so `on_frame_arrived` can rebuild the incoming frame around a rebased timestamp and so `fit` can allocate on that same device. *Why a `Context<()>` and not two typed fields:* the `windows` crate that names `ID3D11Device`/`ID3D11DeviceContext` here is windows-capture's own (0.61), a different version from this crate's `windows` (0.58), so those two types cannot be written in a field declaration - while `windows_capture::capture::Context<()>` can, and its `pub` fields carry the values with their types inferred. (`frame_scaler.rs`, which *does* have to name them, reaches them through the `wgc_windows` alias of that same 0.61 package.)
- `scratch: Vec<u8>` - the readback buffer `Frame::new` requires. Never touched: the rebuilt frame only ever reaches `send_frame`, which reads its surface and its timestamp and nothing else.
- `enc_dims: (u32, u32)` - the size the encoder was actually built for, i.e. the first frame's (or the seed's) rounded down to even (H.264 4:2:0 has no odd sizes; an odd 1697x955 window capture used to come out as an odd-sized file whose nv12 frames no longer measured `w*h*3/2` bytes on decode, so the export slid and sheared, 2026-09-14). Every later frame is either exactly this size or is fitted into it.
- `fit: FrameFit` (`frame_scaler.rs`) - the fixed canvas every LATER size is scaled into. Costs nothing until the capture actually resizes: it builds itself on the first mismatched frame.
- `on_size: SizeHook` - taken and called by the first `on_frame_arrived`; `None` from then on.

## Cap::new

```rust
fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error>
```

Builds the handler on the capture thread, from the `CapFlags` the `Settings` carried and the D3D device/context the crate just created.

A COLD start has `seed: None`: no encoder, `enc_dims` of `(0, 0)`, and a fresh `PauseClock` over the shared ledger - `on_frame_arrived` fills all three from the first real frame. A SEEDED start is the replacement capture of a display switch (`gpu_restart.rs`): it inherits that capture's open encoder, the size it was built for and its `PauseClock`, so the PTS base and `sync.json`'s clock carry straight over and no second encoder is ever built.

`FrameFit` is deliberately NOT inherited: it allocates its canvas and its video-processor chain on the capture's own D3D11 device, and a restart creates a new one. The new fit builds itself on the first frame whose size differs from `enc_dims` - which, on a switch to a differently-sized display, is the very first frame.

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

Encodes one frame, dropping it only when the recording clock or the encoder says it does not belong in the file. The very first frame also reports its raw `width()`/`height()` through `on_size` (taken, so once per capture, before anything else happens to the frame) - the true size of a switched-to target, for `switch_display`'s record.

### Implementation

1. If there is no encoder yet, this is the FIRST frame of a cold-started capture: take `enc_dims` from *its* size, rounded down to even, and build the encoder for that. A seeded capture (a display switch) already has both and skips this entirely, so the new display's frames go straight to step 4 and are fitted into the take's original canvas exactly like a window resize (an odd frame is then fitted into the even canvas by step 3, with no bars: one column or row of a screen capture is invisible). *Why not from the capture target:* `GetWindowRect` on Win10/11 includes the invisible DWM resize margins the WGC surface does not have, so an encoder sized from it mismatched every single frame - dropping them emptied the sink ("no samples were processed") and encoding them anyway made the encoder read each row at the wrong stride and write magenta/green video. Sizing from the surface itself makes the common case exact.
2. Read `clock.now_ms()` and the `paused` flag, and ask `pause_clock.tick`. `None` (paused, or no advance) returns `Ok(())` with nothing recorded and nothing encoded.
3. If the encoder is already gone (`on_closed` finalized the MP4 because the OS ended the capture), return `Ok(())` - this frame is not going into the file, so it must not go into `sync.json` either.
4. If the frame's size no longer matches `enc_dims`, fit it: `FrameFit::fit` (`frame_scaler.rs`) scales it into a persistent D3D11 canvas at `enc_dims`, aspect preserved, centred, black bars, one video-processor blit on the GPU - and the canvas' surface and texture are used in place of the frame's own. *Why a fit and not an end or a skip:* a browser tab switch toggles Chrome's bookmarks bar, which resizes the capture mid-take. Ending the take there is what made recording a browsing session impossible; skipping the frame instead left the picture frozen on the last good frame for the rest of the recording while the audio and the clock kept running. Only if the fit itself fails does the frame get skipped, which is safe because step 6 then withholds its timestamp too.
5. Rebuild the frame with `Frame::new` around that surface and texture (no readback, no CPU copy) but with `timestamp().Duration = tick.pts_100ns`, at `enc_dims` rather than the frame's own size, and `send_frame` that. *Why the rebased timestamp:* `send_frame` stamps the MF sample with `frame.timestamp() - first_timestamp`, i.e. the raw WGC `SystemRelativeTime` QPC capture instant. That keeps every paused span alive in `video.mp4`'s own PTS while `sync.json`, the WAVs and all the input streams drop it; the export then decodes that file 1:1 against `sync.json`'s clock and gets a pause-length frozen span, everything else running ahead of the picture from the resume on, and a truncated tail (finding C1). The crate exposes no way to override the timestamp on the zero-copy path, but `Frame::new`, `as_raw_surface` and `as_raw_texture` are all public, so the rebuild is ordinary API use.
6. Hand the send's result to `record_if_encoded`, which appends `tick.sync_ms` only if it succeeded, and otherwise prints `[capture] encoder rejected a frame at <ms>` to stderr (2026-09-14: a display switch lost every frame after its first without a word); a frame the fitter cannot place prints `[capture] no fit` the same way.

*On the ordering of 2 and 4:* the pause tick runs before the fit so a paused frame never costs a GPU blit. A frame that ticks and is then skipped because the fit failed has advanced `PauseClock`'s `last_ms` without being encoded, which is harmless - that value only rejects a non-increasing PTS, and `send_frame` re-bases the PTS off the first frame it actually sees.

## Cap::on_closed

```rust
fn on_closed(&mut self) -> Result<(), Self::Error>
```

Runs only when the OS closes the capture (the recorded window closed, the display was unplugged/disabled/slept), NOT on a WM_QUIT stop - `GpuRecorder::stop` finalizes that case. Finalizes the encoder so the MP4 is still valid, then calls `ended(CAPTURE_CLOSED)` **unless `handed_over` is set**: after `take_seed` this handler is only waiting for its thread to exit, its encoder belongs to the replacement capture, and reporting the take as ended here would stop a recording that is still running on the other display.

*Why the notification matters (finding M2):* without it the video simply ends here and `GpuRecorder::stop` later returns the truncated timestamps as a perfectly normal success - so the HUD keeps counting, the mic keeps writing narration, and the whole tail of the session does not exist. The notification is sent even if `finish()` fails, and the finalize error is still returned afterwards.
