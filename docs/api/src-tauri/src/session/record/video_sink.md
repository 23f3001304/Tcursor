# src-tauri/src/session/record/video_sink.rs

One type: the value a stopped capture leaves behind.

`VideoStopped` lives here rather than in `platform/windows/` because it names no OS type and it is what `ports::capture::VideoSink::stop` returns - a port that imported its own return type from a platform adapter would point the dependency the wrong way. The pipeline itself (the `VideoSink` enum, `VideoStart`, `start_video`) is `platform/windows/capture/mod.rs`, and `start_ffmpeg` is `platform/windows/capture/legacy/mod.rs`.

Batch C1 left a `#[cfg(windows)] pub use` here so `recorder.rs`, `recorder_stop.rs` and `switch_display.rs` could keep naming the Windows types while the adapters moved. Batch D deleted it: all three now hold `Box<dyn ports::capture::VideoSink>` and never name a platform type.

## VideoStopped

```rust
pub struct VideoStopped {
    pub frames: u64,
    pub frame_ts: Vec<u64>,
    pub error: Option<String>,
}
```

What a stopped video pipeline leaves behind.

- `frames: u64` - successfully encoded frame count.
- `frame_ts: Vec<u64>` - per-frame capture times (ms, pause-compressed) for `sync.json`.
- `error: Option<String>` - a stop/finalize failure. *Why reported alongside the timestamps instead of replacing them:* a finalize failure used to propagate out of the stop path before `sync.json`, `project.tcursor` and the recents entry were written, so an encoder/mux tail failure (disk full on a long take) left a folder that `open_project`'s `*.tcursor` filter cannot even select and that `build_timeline` would have had to synthesise a timeline for (finding M1). The frames that were recorded exist on disk either way, so the caller writes those files from `frame_ts` first and surfaces `error` after. This is also why `ports::capture::VideoSink::stop` returns `VideoStopped` and not `Result<VideoStopped, _>`: a `Result` at the port would put that bug straight back.
