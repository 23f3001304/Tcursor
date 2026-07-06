# src-tauri/src/audio/capture/audio_source.rs

Defines the `AudioSource` trait and its companion test double `FakeAudioSource`. The file is pure with no I/O and no threading - it exists solely to set the interface contract so audio capture backends and WAV writing can be decoupled without knowing which hardware is underneath.

## AudioSource

```rust
pub trait AudioSource: Send {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}
```

Minimal contract that every audio input must satisfy. `Send` is required so a `Box<dyn AudioSource>` can be moved to a recording thread without a wrapper.

- `sample_rate() -> u32` - Returns the native capture rate in Hz (e.g. 48000). *Why:* the WAV header and encoder must declare the rate so playback speed is correct; querying it once at stream open avoids repeated device round-trips.*
- `channels() -> u16` - Returns the channel count (1 = mono, 2 = stereo). *Why:* the WAV header's channel field drives the player's speaker routing and controls how an encoder mux interleaves the data.*

Both values are expected to be constant for the lifetime of the source.

### Used by

No production callers in `src-tauri/src` reference this trait by name today - it serves as a forward-compatibility contract. The concrete types `CpalMicHandle` and `SystemAudioHandle` each embed a `WavWriter` configured with the same rate/channels at construction, filling the trait's intended role without `dyn` dispatch.

## FakeAudioSource

```rust
pub struct FakeAudioSource { pub sample_rate: u32, pub channels: u16 }
impl AudioSource for FakeAudioSource {
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn channels(&self) -> u16 { self.channels }
}
```

Test double that satisfies `AudioSource` with caller-supplied constants and no hardware access.

- `sample_rate: u32` - *Injected value returned verbatim by `AudioSource::sample_rate()`; `pub` so tests can construct via struct literal with no builder ceremony.*
- `channels: u16` - *Injected value returned verbatim by `AudioSource::channels()`; same rationale.*

### Used by

No production callers. Available to any unit test that needs a concrete `AudioSource` without opening a real device.
