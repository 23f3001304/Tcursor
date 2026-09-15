# src-tauri/src/ports/audio.rs

System-audio loopback, the narrowest of the six ports.

cpal already covers Windows, macOS and Linux for the microphone and `audio/capture/` contains no platform API at all, so the mic path gets no port. The one non-portable thing is the loopback trick: opening the DEFAULT OUTPUT device as an INPUT stream is a WASAPI behaviour. macOS needs ScreenCaptureKit audio or an aggregate device, and Linux a PipeWire or PulseAudio `.monitor` source.

## SystemAudioPort

```rust
pub trait SystemAudioPort: Send + Sync
```

Picks the device to record system audio from, and nothing else.

*Why the port stops at device selection.* Everything after it - the WAV writer, the level meter that feeds the HUD's wave, the pause handling, the sample-format conversion - is shared code with no platform API in it. A port that owned the capture would drag all of that into four copies, one per platform, for the sake of three cpal calls.

## SystemAudioPort::loopback_device

```rust
fn loopback_device(&self) -> Option<(cpal::Device, cpal::SupportedStreamConfig)>
```

The device to open as an input stream for system-audio loopback, and its config.

`None` means this platform cannot capture system audio; the HUD greys the toggle rather than offering a recording that would come back silent. That is a real state, not a defensive branch: it is what the Linux adapter answers until a `.monitor` source is wired up, and what macOS answers without ScreenCaptureKit audio.

`cpal::Device` in the signature is deliberate and is not a platform type leaking through: cpal IS the shared abstraction here, used by the mic path on all three platforms already, so returning one keeps the whole capture side of the port at zero.
