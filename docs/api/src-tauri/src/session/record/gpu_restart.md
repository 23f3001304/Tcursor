# src-tauri/src/session/record/gpu_restart.rs

Moving a running GPU-native capture onto another display (or window) mid-take, without the recorded file learning that it happened - the recorder half of `switch_display` (2026-09-14, `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`).

The encoder is the thing that must NOT restart. `video.mp4` is one H.264 stream at one size with one PTS timeline, and the export decodes it against one `sync.json`; a second encoder would mean a second file, a second size and a second clock. So a switch stops the WGC capture, lifts the open `VideoEncoder`, the size it was built for and the recording clock out of the dying `Cap`, and hands all three to a fresh capture on the new target as an `EncoderSeed`. Frames from the new display are then just a size change, which `frame_scaler` / `frame_fit::letterbox` already fits into the canvas - the same path a browser tab toggling its bookmarks bar takes, and the reason a 16:10 laptop panel switched into a 16:9 take arrives with thin black bars rather than a distortion or a stopped recording.

Split from `gpu_record.rs` and `gpu_frames.rs` because both sit at the 200-line cap.

## EncoderSeed

```rust
pub struct EncoderSeed {
    pub encoder: VideoEncoder,
    pub dims: (u32, u32),
    pub pause_clock: PauseClock,
}
```

The running encode, lifted out of one capture so the next one can continue it. Travels to the replacement capture inside `gpu_frames::CapFlags::seed` and is consumed by `Cap::new`.

- `encoder` - still open, still holding `video.mp4`: nothing calls `finish()` across the switch, which is what keeps the take one file. `VideoEncoder` is `unsafe impl Send` in windows-capture, so it can cross to the new capture thread.
- `dims` - the size the encoder was built for, i.e. the take's canvas. It never changes again for the life of the take; the new display's frames are fitted into it.
- `pause_clock` - the recording clock. Carried, not rebuilt, so three things continue rather than restarting: the PTS base (`PauseClock` rebases the encoder PTS onto the first ENCODED frame, and that frame is on the old display), the monotonic `last_ms` check that rejects a non-increasing PTS, and `sync.json`'s own timeline. A fresh clock here would hand the encoder a PTS of 0 mid-file.

## Cap::take_seed

```rust
pub(super) fn take_seed(&mut self) -> (EncoderSpec, Option<EncoderSeed>)
```

Gives up the encode: marks the handler as handed over and moves the encoder, its size and the recording clock out of it.

Returns the `EncoderSpec` as well as the seed, because a switch made before the very first frame of the take has no encoder yet (`Cap` builds it from the first real frame's size) - the seed is then `None` and the replacement capture has to build one itself from that spec, which carries the fps and the output path. This is the ordinary case for a user who picks the wrong display and corrects it immediately.

### Implementation

1. Set `handed_over`, which `Cap::on_closed` reads: the encoder now belongs to the replacement capture, so if the OS happens to close the old item on the way out, that handler must NOT fire `ended(CAPTURE_CLOSED)` and stop a recording which is still running on the other display.
2. Clone the `EncoderSpec` and copy `enc_dims`.
3. `std::mem::replace` the `PauseClock` with a fresh one over the same `PauseTotals`. *Why a replace and not a move:* the handler stays alive until its capture thread exits and must remain a valid `Cap` until then.
4. `take` the encoder; `map` it into an `EncoderSeed` with the dims and the real clock.

## GpuRecorder::restart

```rust
pub fn restart(self, cfg: GpuStart, target_id: Option<&str>, on_size: SizeHook) -> anyhow::Result<(Self, u32, u32)>
```

Stops this capture and starts a new one on `target_id`, feeding the SAME encoder. Returns the replacement recorder and the new target's `(w, h)` - an estimate of the capture's size (`start_capture`'s window rect or monitor size), not the file's, which stays whatever the take's first frame made it. `on_size` (`gpu_frames::SizeHook`) goes into the new `CapFlags` and is told the replacement capture's first frame size, the true one. `frame_ts` is the same `Arc` throughout, so the new capture appends to the timestamp list the old one was filling and `sync.json` describes the take as one continuous stream.

Consumes `self`, so on any failure the take has no capture left. The caller, `video_sink::VideoSink::switch`, is the one that keeps the take: it grabs `frame_times()` first and leaves `VideoSink::Dead` holding it, so the stop path still writes a truthful `sync.json` for the frames that were recorded. The encoder is finalized on every failure path - explicitly by `finish()` when the stop fails, and by its own `Drop` (which sends end-of-stream and joins the transcode thread) when the replacement capture fails to start - so `video.mp4` is a playable file either way.

### Implementation

1. `control.callback()` for the `Arc<Mutex<Cap>>` BEFORE `stop()` consumes the control handle; it is the crate's own parking_lot mutex, so there is no poison to handle.
2. `take_seed()` under that lock, which both lifts the encode out and disarms the old handler's `on_closed`.
3. `control.stop()` - WM_QUIT plus a join. The crate runs `on_closed` only on an OS-initiated close, never on a WM_QUIT stop, so nothing finalizes the MP4 behind us; and after step 2 the handler has no encoder to finalize in any case. A stop failure finalizes the seed and bails, rather than leaving two captures pointed at one encoder.
4. Build `CapFlags` with `seed`, `on_size` and the same `clock` / `frame_ts` / `paused` / `totals` / `ended` the take started with, and hand them to `gpu_record::start_capture`, which resolves the new target and launches through the same `Settings` path a cold start uses.
