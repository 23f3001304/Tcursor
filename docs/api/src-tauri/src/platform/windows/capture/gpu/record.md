# src-tauri/src/platform/windows/capture/gpu/record.rs

GPU-native recording: feeds each WGC frame's D3D11 surface straight into the `windows-capture` Media Foundation `VideoEncoder` (no GPU->CPU readback), curing game-capture lag. The recorded `video.mp4` is the raw full-res intermediate the export re-composites; per-frame capture timestamps are collected here for `sync.json`. This is the default recording path (picked by `capture::start_video`); the ffmpeg pipe is the fallback. The frame callback itself lives in `gpu/frames.rs`, and restarting a live capture on another display without disturbing the encoder in `gpu/restart.rs`.

## target_bitrate

```rust
pub(crate) fn target_bitrate(w: u32, h: u32) -> u32
```

Target H.264 bitrate (bits/s) for the raw recording intermediate at `w`x`h`: `w*h*12` clamped to `[8 Mbps, 80 Mbps]`. Deliberately generous - the export re-encodes this, so compression loss must not be baked into the master - but capped so 4K stays reasonable (~25 Mbps at 1080p; the 80 Mbps cap binds at 4K).

### Behaviors
- `bitrate_scales_with_pixels_and_clamps`: 1920x1080 -> 24_883_200; 320x240 -> 8_000_000 (min clamp); 3840x2160 -> 80_000_000 (max clamp).

## video_settings

```rust
fn video_settings(w: u32, h: u32, fps: u32) -> VideoSettingsBuilder
```

H.264 video settings for the GPU encoder at `w`x`h`@`fps`, at `target_bitrate(w, h)`.

## encoder

```rust
fn encoder(w: u32, h: u32, fps: u32, video_path: &str) -> anyhow::Result<VideoEncoder>
```

The MP4 encoder for a `w`x`h`@`fps` capture writing `video_path`. Audio is disabled - the mic and system-audio WAVs are captured separately and muxed at export. One function so the three capture-target branches in `start` do not each spell out the same four-argument construction.

## GpuRecorder

```rust
pub struct GpuRecorder {
    pub(super) control: CaptureControl<Cap, anyhow::Error>,
    pub(super) frame_ts: FrameTimes,
}
```

A live GPU-native recording. The WGC capture + MF encode run together on the crate's own thread: the `Cap` handler (`gpu/frames.rs`) owns the `VideoEncoder` and encodes each frame's surface in `on_frame_arrived`, taking the frame's place on the recording clock from a `PauseClock` over the shared `PauseTotals` ledger - which both drops frames while paused AND rebases the encoder's PTS, so `video.mp4`'s own timeline is `sync.json`'s timeline. This struct holds the control handle and the shared per-frame timestamps.

- `frame_ts: FrameTimes` - the same `Arc` the handler pushes into, so the timestamps survive independently of the encoder. *Why that matters:* `stop` can hand them back even when the MP4 finalize fails, which is what lets the stop path still write a truthful `sync.json` (finding M1). The same `Arc` also survives a mid-take display switch (`gpu::restart::GpuRecorder::restart` moves it to the replacement recorder), so the take reads back as one continuous stream.

Both fields are `pub(super)` so `gpu/restart.rs` can take the recorder apart and put a replacement together.

## GpuStart

```rust
pub struct GpuStart {
    pub clock: Arc<dyn Clock>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}
```

Everything `start` and `restart` need that isn't the capture target or the output path, bundled so `start_capture` takes one argument for all of it.

- `totals: Arc<PauseTotals>` - the recorder's pause ledger, the same `Arc` every input tracker holds. *Why:* the C1 fix - the encoded PTS is compressed by this ledger, not by an accumulator of its own.
- `ended: Notify` - called if the OS closes the capture (`CAPTURE_CLOSED`, from `Cap::on_closed`) or the capture's own dimensions change mid-record (`DISPLAY_CHANGED`, from `Cap::on_frame_arrived` via `DimGuard` - see `mod.rs`).

## launch

```rust
fn launch<T: TryIntoCaptureItemWithType + Send + 'static>(item: T, cfg: &GpuStart, flags: CapFlags) -> anyhow::Result<CaptureControl<Cap, anyhow::Error>>
```

Starts one WGC capture of `item` with `flags`, on the crate's own thread. The single place a `Settings` is built: monitors, windows and the primary-display fallback differ only in the item they hand in, and a seeded restart goes through exactly the same call as a cold start, so the cursor setting, the border setting and the minimum-update-interval floor cannot drift apart between the two. Generic over the item type because `Settings` is - `Monitor` and `Window` share no common type, only the `TryIntoCaptureItemWithType` trait.

## start_capture

```rust
pub(super) fn start_capture(cfg: &GpuStart, target_id: Option<&str>, flags: CapFlags) -> anyhow::Result<(CaptureControl<Cap, anyhow::Error>, u32, u32)>
```

Resolves `target_id` (`window:0x…`, `display:N`, or `None`/unparseable for the primary display) and starts capturing it with `flags`. Returns the control handle and the target's nominal `(w, h)`.

That size is only an ESTIMATE of what WGC will deliver - `GetWindowRect` includes the invisible DWM resize margins the capture surface does not have - which is why `Cap` sizes the encoder from the first real frame instead and this value is used only for the caller's log line and its `ScreenInfo`. `flags` is taken by value rather than as a closure because a restart's `CapFlags` carries a non-clonable `EncoderSeed`: each branch moves it exactly once and then returns.

## GpuRecorder::start

```rust
pub fn start(cfg: GpuStart, target_id: Option<&str>, video_path: &str) -> anyhow::Result<(Self, u32, u32)>
```

Starts GPU-native capture+encode of the monitor or application window named by `target_id` to `video_path` (H.264 MP4). Returns the recorder plus the captured `(w, h)`. Errors - so the caller can fall back to ffmpeg - when the monitor, the MF H.264 encoder, or the capture cannot initialize.

The encoder is NOT built here (`seed: None`, `Cap` builds it from the first real frame); this only allocates the shared `frame_ts` and carries the `EncoderSpec` the handler will need.

## GpuRecorder::frame_times

```rust
pub fn frame_times(&self) -> FrameTimes
```

The frame timestamps collected so far, as the shared `Arc` rather than a snapshot. Used by `capture::VideoSink::switch`, which has to give the recorder up to `restart` and needs something that will still be filled by the old capture's last frames on its way out - a snapshot taken before the restart would silently shorten `sync.json` on the failure path.

## GpuRecorder::stop

```rust
pub fn stop(self) -> VideoStopped
```

Stops capture and finalizes the MP4: `CaptureControl::stop` posts WM_QUIT and joins the capture thread, then the encoder is finalized **explicitly** (`finish()`) - the crate does NOT call `on_closed` on a WM_QUIT stop, and the encoder's `Drop` would otherwise swallow a tail encode/mux failure on the unrecoverable master.

Returns a `VideoStopped` rather than a `Result`: the per-frame capture timestamps live in their own `Arc` and are handed back in every case, so a capture-stop or finalize failure (reported in `VideoStopped::error`) still leaves the caller able to write `sync.json`, the `.tcursor` manifest and the recents entry for the frames that were recorded. If the OS already closed the capture, `on_closed` will have taken the encoder, and the `finish()` step is simply skipped.
