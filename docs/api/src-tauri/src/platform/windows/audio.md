# src-tauri/src/platform/windows/audio.rs

`SystemAudioPort` on Windows: the WASAPI loopback trick, which is opening the DEFAULT OUTPUT device as an input stream.

This is now the ONE place that trick lives. Batch A had to restate the three cpal calls here because they were inline inside `audio::capture::system_audio::SystemAudio::loopback`; cross-platform Phase 1, Batch C3 (2026-09-15) deleted that copy, so `loopback` asks `crate::platform::loopback_device()` for the device and keeps everything that is genuinely shared - the WAV writer, the level meter, the pause handling, both sample-format arms. The microphone path has no port and was not touched: cpal covers it on all three platforms with no Windows API at all.

## Win32SystemAudio

```rust
pub struct Win32SystemAudio;
```

Windows system audio: the default output device, opened as an input.

## Win32SystemAudio::loopback_device

```rust
fn loopback_device(&self) -> Option<(cpal::Device, cpal::SupportedStreamConfig)>
```

`cpal::default_host().default_output_device()` plus its `default_output_config()`, both as `Option`.

*Why `Option` and not the `anyhow::Result` the live code uses.* Today a missing output device is an error string on the recording path (`"no default output device"`). At the port, "this platform cannot capture system audio" is a permanent property the HUD greys a toggle for, not a per-take failure, and the two cases are indistinguishable from outside - so the port answers the question it is actually asked. The adapter that eventually owns the capture still reports a real open failure through its own path.
