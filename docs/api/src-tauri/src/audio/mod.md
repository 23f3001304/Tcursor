# src-tauri/src/audio/mod.rs

MODULE OVERVIEW: The `audio` module captures microphone and desktop loopback audio to separate WAV files during a recording session. It is organized around a thin abstraction layer and two concrete backends: `audio_source` defines the `AudioSource` trait that decouples capture metadata from WAV encoding, `wav_writer` wraps hound to produce correctly-finalized WAV files, `level` measures what the callbacks are hearing, and `cpal_mic` and `system_audio` implement the two CPAL-backed capture paths. The key data flow is that each backend creates a `WavWriter` at stream-open time, feeds it i16 samples inside the CPAL callback, and calls `finalize` on stop so the RIFF header is correct. Pause support is shared across both backends via an `Arc<AtomicBool>` flag that causes the callback to drop samples without stopping the stream.

## level

The audio-level tap behind the HUD's wave meter. Key items: `block_rms_f32` / `block_rms_i16` (one callback block's RMS in 0..1, identical for both sample formats), `LevelSlot` (a lock-free peak slot the realtime callback pushes into and the owning capture thread drains every ~50ms - a callback may not block, allocate or emit), `AudioLevel` (the `audio-level` event payload). Shared by both backends, and the one piece of the audio module the frontend reads live.

## wav_writer

Thin wrapper around `hound::WavWriter` that fixes encoding to 16-bit signed integer PCM and exposes create, write, and finalize operations with a uniform `std::io::Error` return type. Key items: `WavWriter::create` (opens a new WAV file with fixed bit depth), `WavWriter::write` (appends a slice of i16 samples), `WavWriter::finalize` (flushes the hound buffer and writes correct RIFF chunk-size fields).

