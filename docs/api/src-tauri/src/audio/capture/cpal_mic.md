# src-tauri/src/audio/capture/cpal_mic.rs

Opens a microphone via CPAL, converts incoming samples to i16, and streams them into a WAV file while supporting pause and latency-corrected start-time stamping. The key property is the first-sample timestamp: it subtracts the device's reported input latency so the mic track aligns to when sound was actually captured, not when the OS callback fired, eliminating a class of A/V drift without a manual offset.

## capture_ms

```rust
fn capture_ms(now_ms: u64, info: &cpal::InputCallbackInfo) -> u64
```

Private helper. Computes the estimated capture time (ms) of a callback's first sample by subtracting the device's input-to-callback latency from the current clock reading.

### Inputs

- `now_ms: u64` - Wall-clock reading at callback invocation, from `Clock::now_ms()`. *Why:* provides the reference instant from which latency is subtracted.*
- `info: &cpal::InputCallbackInfo` - CPAL callback metadata. *Why:* exposes `timestamp().capture` and `timestamp().callback`; their difference is the hardware input latency. When the backend reports no timestamp, `duration_since` returns `Err` and `unwrap_or_default` yields zero, so the function falls back to `now_ms` with no underflow risk.*

### Returns

`u64` milliseconds - estimated time the first sample was captured by the microphone hardware. Uses `saturating_sub` so a zero or stale latency value cannot underflow.

### Implementation

1. Call `info.timestamp()` to get the CPAL timestamp pair (`capture`, `callback`).
2. Compute `lat = callback.duration_since(capture).unwrap_or_default()`. *Why `unwrap_or_default`: some WASAPI/CoreAudio backends report no capture timestamp; defaulting to zero latency is better than panicking or skipping the stamp.*
3. Return `now_ms.saturating_sub(lat.as_millis() as u64)`.

## CpalMicHandle

```rust
pub struct CpalMicHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}
```

Owned handle to an active microphone capture stream. Dropping `stream` stops CPAL callbacks; `stop()` additionally finalizes the WAV file.

- `stream: cpal::Stream` - *Live CPAL input stream. CPAL stops calling the callback when `Stream` is dropped, tying stream lifetime to this handle.*
- `writer: Arc<Mutex<Option<WavWriter>>>` - *Shared with the capture callback so samples reach the WAV from the audio thread. The `Option` lets `stop()` take ownership for `finalize()` without leaving a dangling reference inside the `Arc`.*

### Used by

- `session/recorder_threads.rs` - calls `CpalMic::open` and stores the returned handle; calls `stop()` when recording ends.

## CpalMicHandle::stop

```rust
pub fn stop(self) -> std::io::Result<()>
```

Stops audio capture and writes the WAV header with the correct data-length fields.

### Implementation

1. Drop `self.stream`. *Why drop first:* CPAL stops calling the callback immediately, so no new writes can race with the finalize below.*
2. Lock `self.writer` and call `.take()` to replace the inner value with `None` and obtain the `WavWriter`. If already taken, finalization is skipped silently.
3. Call `WavWriter::finalize()` to flush the hound buffer and write the RIFF chunk-size fields.

### Returns

`std::io::Result<()>` - propagates any I/O error from WAV finalization. Stream teardown errors are not surfaced.

## CpalMic

```rust
pub struct CpalMic;
```

Stateless factory for microphone capture streams. All configuration is passed to `open`; all state lives in the returned `CpalMicHandle`.

### Used by

- `session/recorder_threads.rs` - calls `CpalMic::open` on the recording thread to start mic capture.

## CpalMic::open

```rust
pub fn open(
    device_name: Option<&str>,
    wav_path: &str,
    paused: Arc<AtomicBool>,
    started: Arc<AtomicU64>,
    clock: Arc<dyn Clock>,
    level: Option<Arc<LevelSlot>>,
) -> anyhow::Result<CpalMicHandle>
```

Opens a named or default input device, builds an audio callback that writes samples to WAV, and returns a handle to the live stream.

### Inputs

- `device_name: Option<&str>` - Name of the input device to open. *Why:* lets the caller select a specific microphone from user settings; `None` picks the OS default.*
- `wav_path: &str` - Destination WAV file path. *Why:* microphone audio is written independently from system audio so each can be mixed or muted separately at export.*
- `paused: Arc<AtomicBool>` - Shared pause flag. *Why:* when set the callback drops incoming samples so paused time is excluded from the WAV without stopping and restarting the stream.*
- `started: Arc<AtomicU64>` - Written exactly once, on the first non-paused callback, with `capture_ms(clock.now_ms(), info)`. *Why:* the recorder reads this atom to know the mic's epoch and compute A/V sync offsets; `0` means "not yet started".*
- `clock: Arc<dyn Clock>` - Provides `now_ms()` inside the callback. *Why injectable:* tests substitute a `FakeClock` to control timing without real hardware.*
- `level: Option<Arc<LevelSlot>>` - Receives each block's RMS for the HUD's live meter. *Why a slot and not a callback that emits:* this runs on a realtime callback thread, which may not block, allocate or emit; `LevelSlot::push` is a single relaxed `fetch_max`, and the owning thread does the emitting. `None` skips the measurement entirely (`default_input`, tests).*

### Implementation

1. Enumerate CPAL input devices via `default_host()`. Match by exact name when `device_name` is `Some`; error if not found. Use `default_input_device()` when `None`; error if absent.
2. Query `default_input_config()` to get `sample_rate` and `channels`.
3. Create `WavWriter` at `wav_path` for the discovered format.
4. Clone `writer` into `w2` for the callback closure.
5. Build the callback depending on `sample_format`:
   - `F32`: if `paused`, return early. If `started == 0`, stamp it with `capture_ms(clock.now_ms(), info)`. Push `block_rms_f32(data)` to `level`. Clamp each sample to `[-1.0, 1.0]`, scale to `i16::MAX`, write via `WavWriter::write`.
   - `I16`: if `paused`, return early. If `started == 0`, stamp. Push `block_rms_i16(data)` to `level`. Write `data` directly via `WavWriter::write`.
   - Other formats: bail with an unsupported-format error before the stream is started. *Why bail before play:* prevents a half-open stream leaking resources.*
6. Call `stream.play()` to begin receiving callbacks.
7. Return `CpalMicHandle { stream, writer }`.

### Returns

`anyhow::Result<CpalMicHandle>` - errors from device enumeration, config query, WAV creation, or stream building are propagated with context.

## CpalMic::default_input

```rust
pub fn default_input(wav_path: &str) -> anyhow::Result<CpalMicHandle>
```

Convenience wrapper around `open` using the system default device, no pause (`AtomicBool::new(false)`), a zero-initialized start counter (`AtomicU64::new(0)`), a real `SystemClock`, and no level slot. Intended for integration tests and simple command-line usage where shared state is not needed.

### Inputs

- `wav_path: &str` - Path for the output WAV file. *Why:* callers that do not need pause or sync coordination still need to direct the output somewhere.*

### Returns

`anyhow::Result<CpalMicHandle>` - same error surface as `open`.
