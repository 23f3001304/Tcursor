# src-tauri/src/capture/mod.rs

MODULE OVERVIEW: The `capture` module defines the data types and interfaces for screen frame capture and delivers frames from the Windows Graphics Capture API to the encoder loop. It is structured around a clean producer-consumer split: `frame` defines the `Frame` struct that is the unit of data exchange, `frame_source` defines the `FrameSource` pull trait that the encoder drives, and `windows_capture` implements that trait on top of the WGC callback API by bridging WGC's push delivery into a synchronous mpsc channel. Row padding is stripped inside the WGC callback so all downstream consumers receive tightly-packed BGRA rows. Shutdown requires two coordinated steps - setting a halt handle and calling a one-shot stopper - to reliably exit the WGC thread without leaking it.

## frame

Defines `Frame`, the fundamental unit of captured screen data: BGRA8 pixels with a session-relative millisecond timestamp, allocated by the WGC callback and consumed by the encoder. Key items: `Frame` struct (`width`, `height`, `bgra` as tight-packed bytes, `ts` as `Timestamp`).

## frame_source

Defines the `FrameSource` pull trait for decoupling any capture backend from the encoder loop, and provides `FakeFrameSource` as a deterministic `VecDeque`-backed test double. Key items: `FrameSource` trait (`dimensions`, `next_frame` blocking pull, `drain_latest` non-blocking latest-frame pull), `FakeFrameSource::new` (constructs the test double from a `Vec<Frame>`).

## windows_capture

Bridges the Windows Graphics Capture API's callback-based frame delivery into the `FrameSource` pull interface via an internal mpsc channel, and handles two-phase shutdown via a halt handle and a one-shot stopper callable. Key items: `WgcFrameSource::for_primary_display` (creates a WGC capture session on the primary monitor and returns a ready `WgcFrameSource`), `WgcFrameSource::halt_handle` (returns the `Arc<AtomicBool>` for signaling stop from the main thread), `WgcFrameSource::take_stopper` (extracts the `WM_QUIT`-posting callable for coordinated WGC thread exit).
