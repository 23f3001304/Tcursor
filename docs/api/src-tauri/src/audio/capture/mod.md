# src-tauri/src/audio/capture/mod.rs

Submodule overviews for the `capture` group.

## cpal_mic

Opens a named or default microphone via CPAL, converts incoming samples to i16, applies latency-corrected start-time stamping to eliminate A/V drift, and supports pause via an `Arc<AtomicBool>`. Key items: `CpalMic::open` (full constructor accepting device name, WAV path, pause flag, start-time atom, and injectable clock), `CpalMicHandle::stop` (drops the stream and finalizes the WAV), `capture_ms` (subtracts device input latency from callback time to produce the true capture timestamp).

## system_audio

Captures desktop loopback audio by opening the default output device as a CPAL input stream, converting samples to i16, and writing them to a WAV file; no latency correction is applied because loopback is already synchronized with the OS audio engine. Key items: `SystemAudio::loopback` (opens the default output device for loopback capture with a pause flag), `SystemAudioHandle::stop` (drops the stream and finalizes the WAV).

## audio_source

Defines the `AudioSource` trait (sample rate and channel count accessors) and a `FakeAudioSource` test double; pure with no I/O. Key items: `AudioSource` trait (`sample_rate`, `channels`), `FakeAudioSource` (injected constants for unit tests).
