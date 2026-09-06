# src-tauri/src/session/record/gpu_frames.rs

The WGC frame callback for the GPU-native path: hardware-encode each arriving frame on the GPU (no readback) with a pause-aware PTS, and record its capture time for `sync.json`. Split from `gpu_record.rs` (which owns the encoder settings and the recorder lifecycle) so both stay under the 200-line cap.

## FrameTimes

```rust
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;
```

Capture times (ms, pause-compressed) of the frames actually encoded, in encode order - i.e. exactly `sync.json`'s `frames[]`. Shared between the capture thread (which pushes) and `GpuRecorder::stop` (which clones it out), so the timestamps outlive the encoder and survive a finalize failure.

## EncoderSpec

```rust
pub struct EncoderSpec {
    pub fps: u32,
    pub path: String,
}
```

How to build the encoder ONCE the real capture size is known. `GpuRecorder::start` cannot build it: `GetWindowRect` and the monitor dimensions are only an *estimate* of what WGC will deliver, so this carries the settings that do not depend on size and `Cap::on_frame_arrived` supplies the size from the first real frame.

## CapFlags

```rust
pub struct CapFlags {
    pub enc: EncoderSpec,
    pub clock: Arc<dyn Clock>,
    pub frame_ts: FrameTimes,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
}
```

Everything `Cap` needs, handed to it through `Settings`' flags slot (the crate constructs the handler itself, on its own thread, so this is the only way in).

- `totals: Arc<PauseTotals>` - the recorder's exact-span pause ledger, shared with every input tracker.
- `ended: Notify` - called from `on_closed` when the OS ends the capture.

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
    enc: EncoderSpec,
    enc_dims: (u32, u32),
    fit: FrameFit,
}
```

The `GraphicsCaptureApiHandler` the crate runs on its capture thread.

- `encoder: Option<VideoEncoder>` - `None` until the first frame arrives and gives it a size, and `None` again after `stop` or `on_closed` has taken it. `pub` because `GpuRecorder::stop` reaches through the crate's `callback()` handle to `take` and finalize it.
- `pause_clock: PauseClock` - the frame's place on the recording clock, from the shared ledger.
- `gfx: Context<()>` - the capture's D3D device + device context, kept so `on_frame_arrived` can rebuild the incoming frame around a rebased timestamp and so `fit` can allocate on that same device. *Why a `Context<()>` and not two typed fields:* the `windows` crate that names `ID3D11Device`/`ID3D11DeviceContext` here is windows-capture's own (0.61), a different version from this crate's `windows` (0.58), so those two types cannot be written in a field declaration - while `windows_capture::capture::Context<()>` can, and its `pub` fields carry the values with their types inferred. (`frame_scaler.rs`, which *does* have to name them, reaches them through the `wgc_windows` alias of that same 0.61 package.)
- `scratch: Vec<u8>` - the readback buffer `Frame::new` requires. Never touched: the rebuilt frame only ever reaches `send_frame`, which reads its surface and its timestamp and nothing else.
- `enc_dims: (u32, u32)` - the size the encoder was actually built for, i.e. the first frame's. Every later frame is either exactly this size or is fitted into it.
- `fit: FrameFit` (`frame_scaler.rs`) - the fixed canvas every LATER size is scaled into. Costs nothing until the capture actually resizes: it builds itself on the first mismatched frame.

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

Encodes one frame, dropping it only when the recording clock or the encoder says it does not belong in the file.

### Implementation

1. If there is no encoder yet, this is the FIRST frame: take `enc_dims` from *its* size and build the encoder for that. *Why not from the capture target:* `GetWindowRect` on Win10/11 includes the invisible DWM resize margins the WGC surface does not have, so an encoder sized from it mismatched every single frame - dropping them emptied the sink ("no samples were processed") and encoding them anyway made the encoder read each row at the wrong stride and write magenta/green video. Sizing from the surface itself makes the common case exact.
2. Read `clock.now_ms()` and the `paused` flag, and ask `pause_clock.tick`. `None` (paused, or no advance) returns `Ok(())` with nothing recorded and nothing encoded.
3. If the encoder is already gone (`on_closed` finalized the MP4 because the OS ended the capture), return `Ok(())` - this frame is not going into the file, so it must not go into `sync.json` either.
4. If the frame's size no longer matches `enc_dims`, fit it: `FrameFit::fit` (`frame_scaler.rs`) scales it into a persistent D3D11 canvas at `enc_dims`, aspect preserved, centred, black bars, one video-processor blit on the GPU - and the canvas' surface and texture are used in place of the frame's own. *Why a fit and not an end or a skip:* a browser tab switch toggles Chrome's bookmarks bar, which resizes the capture mid-take. Ending the take there is what made recording a browsing session impossible; skipping the frame instead left the picture frozen on the last good frame for the rest of the recording while the audio and the clock kept running. Only if the fit itself fails does the frame get skipped, which is safe because step 6 then withholds its timestamp too.
5. Rebuild the frame with `Frame::new` around that surface and texture (no readback, no CPU copy) but with `timestamp().Duration = tick.pts_100ns`, at `enc_dims` rather than the frame's own size, and `send_frame` that. *Why the rebased timestamp:* `send_frame` stamps the MF sample with `frame.timestamp() - first_timestamp`, i.e. the raw WGC `SystemRelativeTime` QPC capture instant. That keeps every paused span alive in `video.mp4`'s own PTS while `sync.json`, the WAVs and all the input streams drop it; the export then decodes that file 1:1 against `sync.json`'s clock and gets a pause-length frozen span, everything else running ahead of the picture from the resume on, and a truncated tail (finding C1). The crate exposes no way to override the timestamp on the zero-copy path, but `Frame::new`, `as_raw_surface` and `as_raw_texture` are all public, so the rebuild is ordinary API use.
6. Hand the send's result to `record_if_encoded`, which appends `tick.sync_ms` only if it succeeded.

*On the ordering of 2 and 4:* the pause tick runs before the fit so a paused frame never costs a GPU blit. A frame that ticks and is then skipped because the fit failed has advanced `PauseClock`'s `last_ms` without being encoded, which is harmless - that value only rejects a non-increasing PTS, and `send_frame` re-bases the PTS off the first frame it actually sees.

## Cap::on_closed

```rust
fn on_closed(&mut self) -> Result<(), Self::Error>
```

Runs only when the OS closes the capture (the recorded window closed, the display was unplugged/disabled/slept), NOT on a WM_QUIT stop - `GpuRecorder::stop` finalizes that case. Finalizes the encoder so the MP4 is still valid, then calls `ended(CAPTURE_CLOSED)`.

*Why the notification matters (finding M2):* without it the video simply ends here and `GpuRecorder::stop` later returns the truncated timestamps as a perfectly normal success - so the HUD keeps counting, the mic keeps writing narration, and the whole tail of the session does not exist. The notification is sent even if `finish()` fails, and the finalize error is still returned afterwards.
