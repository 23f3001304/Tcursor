# src-tauri/src/export/timeline.rs

Builds the per-frame timestamp vector that places every encoded video frame at its true wall-clock time, and records the start offsets of mic and system audio tracks relative to the video. Abstracts over two paths: a precise sync log (recorded at capture time) and a synthesized uniform fallback for older recordings.

## Timeline

```rust
pub struct Timeline {
    pub frames: Vec<u64>,
    pub events_ms: u64,
    pub mic_ms: Option<u64>,
    pub system_ms: Option<u64>,
}
```

The real capture timeline for one project.

- `frames: Vec<u64>` - one entry per encoded video frame; each value is the wall-clock time in milliseconds. Length equals the raw video frame count. *Why:* the export loop uses `frames[i]` to know at which output time each captured frame becomes active, enabling correct rendering of variable-rate capture (frames are not uniformly spaced when the screen is idle).
- `events_ms: u64` - absolute timestamp (ms, same epoch as `frames`) at which the event log starts. *Why:* the exporter subtracts this from output time `t` before querying events, so event timestamps are relative to the video start even when the event clock and video clock diverged.
- `mic_ms: Option<u64>` - start time of the mic audio track, or `None` when no mic file exists. *Why:* the muxer needs to shift the mic stream so it aligns with the first video frame.
- `system_ms: Option<u64>` - start time of the system audio track, or `None` when absent. *Why:* same shift logic as mic.

### Used by

- `src-tauri/src/export/exporter.rs` - `build_timeline` is called once; `tl.frames` drives the frame loop; `tl.mic_ms` / `tl.system_ms` / `tl.events_ms` compute the audio shift and event offset.

## build_timeline

```rust
pub fn build_timeline(paths: &ProjectPaths, log: &EventLog, fps: u32) -> Timeline
```

Constructs a `Timeline` for the given project, using the recorded sync log when available and synthesizing a uniform timeline otherwise.

### Inputs

- `paths: &ProjectPaths` - project folder root. *Why:* all paths (sync.json, video, mic, system audio) are derived from here.
- `log: &EventLog` - the recorded event log. *Why:* used as a fallback reference duration when no audio is present (`log.events.last().t`).
- `fps: u32` - the capture frame rate. *Why:* last-resort denominator when no audio and no video duration probe succeeds; keeps the synthesized timeline from being zero-length.

### Returns

A `Timeline`. Never fails - falls back gracefully through several heuristics.

### Implementation

1. Try `SyncLog::load(&paths.sync())`. If it loads and has at least one frame, return a `Timeline` directly from the sync log (most-precise path).
2. Fallback: probe the frame count from the raw video (`probe_frame_count`; default 1 on failure). Probe the video duration (`probe_duration`).
3. Determine a reference duration via `audio_dur(paths)` (private helper, tries mic then system audio); if no audio, use the last event timestamp; if no events, use the probed video duration; last resort: `count / fps`.
4. Compute effective fps: `efps = count / ref_dur`.
5. Generate `frames`: `(0..count).map(|i| round(i * 1000 / efps))`.
6. Return `Timeline { frames, events_ms: 0, mic_ms: Some(0) if mic exists, system_ms: Some(0) if system exists }`.

### Behaviors worth knowing

- On the fallback path `events_ms` is always `0`, so the exporter aligns events to the start of the video without any offset.
- `mic_ms = Some(0)` and `system_ms = Some(0)` on the fallback path; the muxer then computes audio shift as `0 - video_start`, which correctly shifts audio forward when the video starts after zero.
