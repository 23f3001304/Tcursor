# src-tauri/src/domain/mod.rs

MODULE OVERVIEW: The `domain` module holds the one foundational value type shared across the recording, capture and export layers: `time`, whose `Clock` trait and `Timestamp` newtype are threaded through `capture`, `audio`, `session` and `export` to provide injectable time without coupling any layer to wall-clock hardware. Pure data, no I/O or threading of its own.

**What left (cleanup batch 1, 2026-09-15).** `ids` (`FrameIndex`, `ScreenCoord`, `CanvasCoord`) and `capture_source` (`CaptureSource`) had zero consumers across `src-tauri/src`, `src-tauri/tests` and `build.rs`; both files, their tests and their docs pages were deleted. `FakeClock` went with them for the same reason (see `time.md`).

## time

Defines `Timestamp` (session-relative millisecond newtype), the `Clock` trait for injectable time sources, and `SystemClock` (the wall-clock implementation). Key items: `Clock` trait (`now_ms`), `SystemClock::new` (captures the session epoch at construction), `Timestamp::ZERO` (sentinel for "not yet set").
