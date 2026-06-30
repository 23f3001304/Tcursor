# src-tauri/src/domain/mod.rs

MODULE OVERVIEW: The `domain` module defines the foundational value types shared across the recording, capture, and export layers: time representations, typed identifiers, and capture target descriptors. All three submodules are pure data with no I/O or threading of their own; they exist to establish type-level distinctions that prevent silent bugs (wrong coordinate space, raw u64 compared against a timestamp, etc.). The module's most pervasive contribution is `time`, whose `Clock` trait and `Timestamp` newtype are threaded through `capture`, `audio`, `session`, and `export` to provide injectable, testable time without coupling any layer to wall-clock hardware.

## time

Defines `Timestamp` (session-relative millisecond newtype), the `Clock` trait for injectable time sources, `SystemClock` (wall-clock implementation), and `FakeClock` (atomic test double). Key items: `Clock` trait (`now_ms`), `SystemClock::new` (captures the session epoch at construction), `FakeClock::advance` (atomically increments time for test control), `Timestamp::ZERO` (sentinel for "not yet set").

## ids

Defines typed wrappers for frame counters and coordinate spaces so the compiler rejects accidental mixing of screen-space and canvas-space coordinates at compile time. Key items: `FrameIndex` (monotonic frame counter with `next`), `ScreenCoord` (signed integer screen-space pixel position), `CanvasCoord` (float canvas-space position).

## capture_source

Defines `CaptureSource`, a `Copy`-able domain enum identifying what screen region or target a recording session should capture. Key items: `CaptureSource` enum (`Display(u32)`, `Window(u64)`, `Region { x, y, w, h }`).
