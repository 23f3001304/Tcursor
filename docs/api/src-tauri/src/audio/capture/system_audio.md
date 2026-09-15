# src-tauri/src/audio/capture/system_audio.rs

Captures desktop loopback audio (what the speakers are playing) by opening the platform's loopback device as a CPAL input stream, converting samples to i16, and writing the result to a WAV file. `started` is stamped inside the data callback at the first non-empty packet, mirroring `cpal_mic`'s in-callback pattern - so `system_ms` reflects when samples actually began arriving rather than the (earlier) moment the stream handle was opened. Unlike `cpal_mic`, no device-latency correction is applied to that stamp: CPAL's `InputCallbackInfo` capture/callback timestamps are mic-specific and unused here.

## SystemAudioHandle

```rust
pub struct SystemAudioHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}
```

Owned handle to the active loopback capture stream. Dropping `stream` stops CPAL callbacks; `stop()` additionally finalizes the WAV.

- `stream: cpal::Stream` - *Live CPAL input stream on the output device. Stream lifetime is tied to this handle so capture stops when the handle is dropped or `stop()` is called.*
- `writer: Arc<Mutex<Option<WavWriter>>>` - *Shared with the capture callback so samples reach the WAV from the audio thread. The `Option` lets `stop()` take ownership for finalization without leaving a dangling reference inside the `Arc`.*

### Used by

- `session/recorder_threads.rs` - calls `SystemAudio::loopback` to start system-audio capture and calls `stop()` when the recording ends.

## SystemAudioHandle::stop

```rust
pub fn stop(self) -> std::io::Result<()>
```

Stops loopback capture and flushes the WAV to disk with correct chunk-size fields.

### Implementation

1. Drop `self.stream`. *Why drop first:* CPAL stops calling the callback immediately, so no new writes can race with the finalize below.*
2. Lock `self.writer`, call `.take()` to replace the inner value with `None` and obtain the `WavWriter`. If already taken, finalization is skipped silently.
3. Call `WavWriter::finalize()` to write the RIFF chunk-size fields and flush the hound buffer.

### Returns

`std::io::Result<()>` - propagates WAV finalization errors. Stream teardown errors are not surfaced.

## SystemAudio

```rust
pub struct SystemAudio;
```

Stateless factory for loopback capture. All configuration is passed to `loopback`; all state lives in the returned `SystemAudioHandle`.

### Used by

- `session/recorder_threads.rs` - constructs loopback capture via `SystemAudio::loopback`.

## SystemAudio::loopback

```rust
pub fn loopback(loopback: Option<(cpal::Device, cpal::SupportedStreamConfig)>, wav_path: &str,
    paused: Arc<AtomicBool>, started: Arc<AtomicU64>, clock: Arc<dyn Clock>,
    level: Option<Arc<LevelSlot>>) -> anyhow::Result<SystemAudioHandle>
```

Opens the given device as a loopback input, builds a sample callback that stamps `started` at the first non-empty packet, and starts the stream.

### Inputs

- `loopback: Option<(cpal::Device, cpal::SupportedStreamConfig)>` - the device to open as an input, and its config, from `ports::audio::SystemAudioPort::loopback_device`. *Why handed in rather than looked up here:* opening the DEFAULT OUTPUT device as an input stream is the WASAPI trick, and it is the only non-portable thing in this file - everything below (the WAV writer, the level meter, the pause gate, the format conversion) is shared. Batch C3 moved the lookup into `platform/windows/audio.rs`; Batch D stopped calling it through a free function and made it an argument, threaded down from `start_take` through `recorder_threads::spawn_system_thread`. *Why `Option` and not a required device:* `None` is the port's permanent "this platform cannot capture system audio", and it becomes the same `"no default output device"` error this function has always reported, so the HUD's warning path is unchanged.
- `wav_path: &str` - Destination WAV file path. *Why:* system audio and microphone are captured to separate files so they can be mixed with independent volume controls at export time.*
- `paused: Arc<AtomicBool>` - Shared pause flag. *Why:* when set the callback discards incoming loopback samples so paused time is excluded from the WAV without stopping and restarting the stream.*
- `started: Arc<AtomicU64>` - Written once, in the data callback, on the first non-empty packet with `clock.now_ms()` (compare against `0` then store - the callback runs on one CPAL thread so no `compare_exchange` is needed, matching `CpalMic::open`'s pattern). *Why in-callback and not at open:* stream-open (config query, `WavWriter::create`, `build_input_stream`, `stream.play()`) measurably precedes the first sample; stamping in the callback removes that gap so `system_ms` lines up with when audio actually starts, the same way `cpal_mic`'s `capture_ms` removes device latency.*
- `clock: Arc<dyn Clock>` - Provides `now_ms()` inside the callback. *Why injectable:* a test hands in its own deterministic `Clock` without needing real hardware timing; `SystemClock` is the production instance.*
- `level: Option<Arc<LevelSlot>>` - Receives each block's RMS for the HUD's live meter. Same lock-free push from the realtime callback, and the same reason, as `CpalMic::open`. This is what makes the meter's back stroke real: system-audio levels never reached the frontend before, because a webview has no way to see them.*

### Implementation

1. Take the device and config out of `loopback`, bailing with `"no default output device"` if it is `None` - which is what greys the HUD's system-audio toggle on a platform whose adapter cannot do it.
2. Read `sample_rate` and `channels` off the returned config.
3. Create `WavWriter` at `wav_path` for the discovered format.
4. Clone `writer` into `w2` for the callback closure.
5. Build the callback depending on `sample_format`:
   - `F32`: if `paused`, return early. If `data` is non-empty and `started == 0`, store `clock.now_ms()` into `started`. Push `block_rms_f32(data)` to `level`. Clamp each sample to `[-1.0, 1.0]`, scale to `i16::MAX`, write to WAV.
   - `I16`: if `paused`, return early. Same `started` stamp as `F32`. Push `block_rms_i16(data)` to `level`. Write `data` directly to WAV.
   - Other formats: bail with an unsupported-format error before the stream is started.
6. Call `stream.play()`.
7. Return `SystemAudioHandle { stream, writer }`.

### Returns

`anyhow::Result<SystemAudioHandle>` - errors from device lookup, config query, WAV creation, or stream building are propagated with context.

### Behaviors

The `InputCallbackInfo` parameter is still ignored (`_`) in the callback signature: unlike `cpal_mic`, there is no per-sample device-latency correction applied to the `started` stamp (loopback audio is already in time with the OS audio engine driving the video capture clock, so only the callback-vs-open gap needs correcting, not per-sample hardware latency).

**Poison tolerance on the driver callback (cleanup batch 3, 2026-09-15).** Both stream callbacks take the writer lock with `.lock().unwrap_or_else(|e| e.into_inner())`, the same shape `asr/commands.rs` uses, rather than `.unwrap()`. A writer thread that panicked while holding the mutex used to poison it, and the next audio block would then panic on the driver's own thread mid-take, which the driver reports as a stream error and the take loses its audio from that point. Continuing to write through a poisoned lock is the right call here: the writer's state is a file handle and a sample count, nothing a half-finished write can corrupt beyond the block that was in flight.
