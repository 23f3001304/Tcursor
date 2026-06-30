# src-tauri/src/audio/wav_writer.rs

Thin wrapper around `hound::WavWriter` that fixes the encoding to 16-bit signed integer PCM and exposes the three operations the audio pipeline needs: create, write samples, and finalize. Hound errors are re-wrapped as `std::io::Error` so callers see a uniform error type. All I/O is buffered by hound's internal `BufWriter`; latency per sample write is negligible. Single-threaded - callers hold the writer behind `Arc<Mutex<Option<WavWriter>>>` to share it with audio callbacks.

## WavWriter

```rust
pub struct WavWriter { inner: hound::WavWriter<std::io::BufWriter<std::fs::File>> }
```

Write-only handle to an open WAV file. The `inner` field is private; all interaction goes through the methods below.

- `inner: hound::WavWriter<BufWriter<File>>` - *Buffered hound writer that owns the file handle. Buffers sample writes in memory and leaves the RIFF chunk-size fields at zero until `finalize` is called. Dropping without finalizing leaves those fields incorrect, producing an unreadable or truncated file.*

### Used by

- `audio/cpal_mic.rs` - creates a `WavWriter` in `CpalMic::open`; writes samples from the CPAL callback via `Arc<Mutex<Option<WavWriter>>>`; finalizes via `CpalMicHandle::stop`.
- `audio/system_audio.rs` - same pattern for the loopback stream in `SystemAudio::loopback` and `SystemAudioHandle::stop`.

## WavWriter::create

```rust
pub fn create(path: &str, sample_rate: u32, channels: u16) -> std::io::Result<Self>
```

Creates a new WAV file at `path` and returns a writer ready to accept samples.

### Inputs

- `path: &str` - File system path for the output WAV. *Why:* each audio source (microphone, loopback) supplies its own path so the two tracks stay in separate files that can be muxed independently at export.*
- `sample_rate: u32` - Sample rate in Hz (e.g. 48000). *Why:* written into the WAV header so any player knows the correct playback speed without guessing.*
- `channels: u16` - Number of interleaved channels (1 = mono, 2 = stereo). *Why:* the WAV header's channel field drives the player's speaker routing and controls how the muxer interleaves the data at export.*

### Implementation

1. Build a `hound::WavSpec` with `bits_per_sample = 16` and `sample_format = SampleFormat::Int`. *Why fixed 16-bit int:* both capture backends convert to i16 before calling this layer, keeping the WAV format uniform regardless of the device's native format.*
2. Call `hound::WavWriter::create(path, spec)`, mapping any hound error to `std::io::Error` with kind `Other`.
3. Return `Self { inner }`.

### Returns

`std::io::Result<Self>` - fails if the path is not writable or hound cannot write the initial RIFF header.

## WavWriter::write

```rust
pub fn write(&mut self, samples: &[i16])
```

Appends a slice of interleaved i16 PCM samples to the WAV file.

### Inputs

- `samples: &[i16]` - Interleaved PCM samples for all channels, in the order produced by CPAL. *Why i16:* both capture backends convert to i16 before calling `write`, keeping this layer format-agnostic. Channel interleaving is performed by the caller; no reordering happens here.*

### Implementation

Iterates `samples` and calls `self.inner.write_sample(s)` for each, discarding individual errors with `let _ = ...`. *Why discard:* audio callbacks cannot block or propagate errors; persistent write failures surface instead at `finalize` time when the hound buffer is flushed.*

## WavWriter::finalize

```rust
pub fn finalize(self) -> std::io::Result<()>
```

Consumes the writer, flushes the hound buffer, seeks back to the RIFF header, and writes the correct chunk-size fields. Must be called to produce a valid, playable WAV file.

### Implementation

Delegates to `self.inner.finalize()`, mapping any hound error to `std::io::Error` with kind `Other`.

### Returns

`std::io::Result<()>` - fails if the final flush or header-seek fails (e.g. the disk is full, or the file was externally deleted between `create` and `finalize`).

### Behaviors

- `writes_readable_wav` - creates a 48000 Hz mono WAV, writes five i16 samples including boundary values `32767` and `-32768`, calls `finalize`, reopens with `hound::WavReader`, and asserts that sample rate, channel count, and total sample count are all correct.
