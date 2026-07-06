# src-tauri/src/audio/capture/system_audio.rs

Captures desktop loopback audio (what the speakers are playing) by opening the default output device as a CPAL input stream, converting samples to i16, and writing the result to a WAV file. Unlike `cpal_mic`, no latency correction or start-time stamp is applied because loopback audio is already synchronized with the OS audio engine that drives the video capture clock.

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
pub fn loopback(wav_path: &str, paused: Arc<AtomicBool>) -> anyhow::Result<SystemAudioHandle>
```

Opens the default output device as a loopback input, builds a sample callback, and starts the stream.

### Inputs

- `wav_path: &str` - Destination WAV file path. *Why:* system audio and microphone are captured to separate files so they can be mixed with independent volume controls at export time.*
- `paused: Arc<AtomicBool>` - Shared pause flag. *Why:* when set the callback discards incoming loopback samples so paused time is excluded from the WAV without stopping and restarting the stream.*

### Implementation

1. Call `cpal::default_host().default_output_device()`. *Why the output device:* WASAPI loopback capture works by opening the output device as an input stream, letting the OS pipe the mixer output to the callback.* Bail if absent.
2. Query `default_output_config()` to get `sample_rate` and `channels`.
3. Create `WavWriter` at `wav_path` for the discovered format.
4. Clone `writer` into `w2` for the callback closure.
5. Build the callback depending on `sample_format`:
   - `F32`: if `paused`, return early. Clamp each sample to `[-1.0, 1.0]`, scale to `i16::MAX`, write to WAV.
   - `I16`: if `paused`, return early. Write `data` directly to WAV.
   - Other formats: bail with an unsupported-format error before the stream is started.
6. Call `stream.play()`.
7. Return `SystemAudioHandle { stream, writer }`.

### Returns

`anyhow::Result<SystemAudioHandle>` - errors from device lookup, config query, WAV creation, or stream building are propagated with context.

### Behaviors

No start-time stamp or latency correction is applied. The `InputCallbackInfo` parameter is ignored (`_`) in the callback because loopback audio is already in time with the OS audio engine driving the video capture clock; no manual alignment is needed, unlike the microphone path in `cpal_mic`.
