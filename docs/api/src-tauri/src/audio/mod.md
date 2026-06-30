# src-tauri/src/audio/mod.rs

MODULE OVERVIEW: The `audio` module captures microphone and desktop loopback audio to separate WAV files during a recording session. It is organized around a thin abstraction layer and two concrete backends: `audio_source` defines the `AudioSource` trait that decouples capture metadata from WAV encoding, `wav_writer` wraps hound to produce correctly-finalized WAV files, and `cpal_mic` and `system_audio` implement the two CPAL-backed capture paths. The key data flow is that each backend creates a `WavWriter` at stream-open time, feeds it i16 samples inside the CPAL callback, and calls `finalize` on stop so the RIFF header is correct. Pause support is shared across both backends via an `Arc<AtomicBool>` flag that causes the callback to drop samples without stopping the stream.

## wav_writer

Thin wrapper around `hound::WavWriter` that fixes encoding to 16-bit signed integer PCM and exposes create, write, and finalize operations with a uniform `std::io::Error` return type. Key items: `WavWriter::create` (opens a new WAV file with fixed bit depth), `WavWriter::write` (appends a slice of i16 samples), `WavWriter::finalize` (flushes the hound buffer and writes correct RIFF chunk-size fields).

## audio_source

Defines the `AudioSource` trait (sample rate and channel count accessors) and a `FakeAudioSource` test double; pure with no I/O. Key items: `AudioSource` trait (`sample_rate`, `channels`), `FakeAudioSource` (injected constants for unit tests).

## cpal_mic

Opens a named or default microphone via CPAL, converts incoming samples to i16, applies latency-corrected start-time stamping to eliminate A/V drift, and supports pause via an `Arc<AtomicBool>`. Key items: `CpalMic::open` (full constructor accepting device name, WAV path, pause flag, start-time atom, and injectable clock), `CpalMicHandle::stop` (drops the stream and finalizes the WAV), `capture_ms` (subtracts device input latency from callback time to produce the true capture timestamp).

## system_audio

Captures desktop loopback audio by opening the default output device as a CPAL input stream, converting samples to i16, and writing them to a WAV file; no latency correction is applied because loopback is already synchronized with the OS audio engine. Key items: `SystemAudio::loopback` (opens the default output device for loopback capture with a pause flag), `SystemAudioHandle::stop` (drops the stream and finalizes the WAV).
