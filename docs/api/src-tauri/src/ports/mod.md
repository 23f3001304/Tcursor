# src-tauri/src/ports/mod.rs

MODULE OVERVIEW: The `ports` module is the operating-system boundary, expressed as traits and plain data. Nothing under it names a Windows, macOS or Linux type, so the recording core can be read - and one day tested headlessly against a mock - without an OS in the room. Six traits across four files: capture and its live sink, the three input streams, the small desktop facts, and system-audio loopback. The adapters that satisfy them live in `crate::platform`, and `crate::platform::current()` is the one place a `#[cfg]` picks a set.

Every signature here is shaped by a bug the recorder already fixed, and the doc comments in the source say which. The three that matter most: `VideoSink::stop` returns a value rather than a `Result` (a finalize failure must not stop `sync.json` being written), no port has `pause`/`resume` (the pause instant is stamped once under the recorder's lock and every stream subtracts that one ledger), and `VideoSink::switch` carries a first-frame-size hook (the render crops the fitted picture by the capture's real size, not the window's rectangle).

## capture

Putting screen pixels in a file: which targets exist, where they sit, and the live pipeline that writes `video.mp4`. Key items: `TargetId` (`Primary` / `Display(usize)` / `Window(u64)`, with the string and `Option<&str>` conversions that replace three hand-written parse sites), `TargetKind`, `CaptureTarget`, `CaptureGeometry`, `CaptureRequest`, `FirstFrameSize`, `VideoSink` (`switch` / `stop` / `supports_switch`), `CapturePort` (`list_targets` / `bounds` / `start`).

## input

The three input streams a take records, as three ports rather than one bundle. Key items: `PointerPort` (`set_remap`, `stop`), `HotkeyPort`, `CursorShapePort`, `InputPort` (`pointer` / `hotkeys` / `cursor_shapes`, each returning a stream that is already running).

## system

The small OS facts that are not capture, input or audio. Key items: `WindowHandle` (an opaque `isize`, resolved from a `tauri::WebviewWindow` at the composition root so no port reaches for a specific window), `SystemPort` (`primary_refresh_hz`, `os_prefers_dark`, `exclude_from_capture`).

## audio

System-audio loopback, the narrowest of the ports: cpal already covers all three platforms for the microphone, so only the "open the default OUTPUT device as an input" trick needs one. Key item: `SystemAudioPort::loopback_device`, whose `None` means the platform cannot capture system audio at all and the HUD greys the toggle.
