# src-tauri/src/platform/windows/mod.rs

MODULE OVERVIEW: The Windows adapters, one per port, as cross-platform Phase 1 Batch C left them (2026-09-15): `capture/` (Batch C1: the WGC and D3D11 pipelines, target enumeration and bounds, `TargetId` parsed once), `input/` (Batch C2: the `WH_MOUSE_LL` hook with its now-private `SINK`, the hotkey poll, the 16 ms cursor-shape poll, the GDI cursor bitmap), `system.rs` (Batch C3: refresh rate, dark mode, capture exclusion with the affinity read back) and `audio.rs` (Batch C3: the WASAPI loopback device). Every Win32 body lives under this module now; the `win/` tree is gone. What the recorder still calls by the old names is a set of `#[cfg(windows)]` re-exports and type aliases in `session/record/`, `capture/`, `events/track/` and `actions/`, which Batch D removes when `Running` holds the boxed ports and `lib.rs` builds one `Platform` in `setup`.

Batch A had landed these as forwarders to free functions, split by port only for the old line cap; the bodies moved in behind the same trait surface, which is what the seam bought.

## capture

`CapturePort` and `VideoSink` over Windows Graphics Capture, through whichever of the two pipelines `start_video` picks. Key items: `Win32Capture`, `Win32Sink` (which also owns the stop flag `CaptureRequest` does not carry).

No longer a forwarder: Batch C1 moved the bodies in, so this is a directory - `capture/mod.rs` (the port impls, the pipeline enum and `start_video`), `capture/target.rs` (bounds and enumeration), `capture/gpu/` (the default Media Foundation path) and `capture/legacy/` (the ffmpeg compatibility path). What is left in the portable tree is `VideoStopped` plus three `#[cfg(windows)]` module re-exports that keep `recorder.rs`, `switch_display.rs` and `tests/manual_capture.rs` compiling until Batch D.

## input

`InputPort` and the three stream ports. Batch C2 moved the bodies in, so this is a directory now: `pointer.rs` (the `WH_MOUSE_LL` hook, the one stamp site, the now-private `SINK`), `hotkeys.rs` (the `GetAsyncKeyState` poll), `cursor.rs` (the 16 ms `GetCursorInfo` poll) and `bitmap.rs` (one `HCURSOR` to one RGBA sprite). Key items: `Win32Input`, `Win32Pointer`, `Win32Hotkeys`, `Win32CursorShapes`.

## system

`SystemPort`: the display refresh rate (`EnumDisplaySettingsW`), the registry dark-mode read, and the capture-exclusion affinity, which is READ BACK after it is set. Key item: `Win32System`. The portable fallbacks (60 Hz, light, `false`) live in `platform/mod.rs`, because this module does not compile off Windows.

## audio

`SystemAudioPort`: the WASAPI loopback trick, which is opening the default OUTPUT device as an input. The one place it lives. Key item: `Win32SystemAudio`.
