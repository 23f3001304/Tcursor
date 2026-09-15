# src-tauri/src/capture/mod.rs

MODULE OVERVIEW: The portable half of frame capture: the data type frames are exchanged as and the pull trait the encoder drives. `frame` defines the `Frame` struct (tight-packed BGRA plus a session-relative timestamp), and `frame_source` defines the `FrameSource` trait with a deterministic `FakeFrameSource` double, which is what lets `RecordingSession` be tested with no OS capture at all.

The Windows implementation of `FrameSource` moved to `platform/windows/capture/legacy/wgc_source.rs` in Batch C1; nothing platform-specific is left in this folder, and Batch D deleted the `crate::capture::windows_capture` re-export that kept the old path resolving. `tests/manual_capture.rs`, its last caller, names the adapter path directly now, so what remains here compiles for any target.

## frame

Defines `Frame`, the fundamental unit of captured screen data: BGRA8 pixels with a session-relative millisecond timestamp, allocated by the WGC callback and consumed by the encoder. Key items: `Frame` struct (`width`, `height`, `bgra` as tight-packed bytes, `ts` as `Timestamp`).

## frame_source

Defines the `FrameSource` pull trait for decoupling any capture backend from the encoder loop, and provides `FakeFrameSource` as a deterministic `VecDeque`-backed test double. Key items: `FrameSource` trait (`dimensions`, `next_frame` blocking pull, `drain_latest` non-blocking latest-frame pull), `FakeFrameSource::new` (constructs the test double from a `Vec<Frame>`).
