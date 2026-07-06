# src-tauri/src/session/sync.rs

Per-recording timing log that anchors every captured track to the shared capture clock. Written once at the end of `stop_recording` and read back by the exporter to reconstruct the real frame timeline and align audio offsets, independent of the labeled fps on any track.

## SyncLog

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncLog {
    pub frames: Vec<u64>,
    pub events_ms: u64,
    #[serde(default)]
    pub mic_ms: Option<u64>,
    #[serde(default)]
    pub system_ms: Option<u64>,
}
```

Timing metadata for one recording, persisted as `sync.json` in the project folder.

- `frames: Vec<u64>` - capture time (ms) of each encoded video frame in encode order, on the shared capture clock. *Why per-frame:* the encoder timestamp may not match wall time (game-mode uses uniform synthetic timestamps); this field stores the real capture time so the exporter can place each frame correctly in the timeline even when the source FPS was variable or paused.
- `events_ms: u64` - value of the capture clock at the moment `MouseTracker` was started. *Why:* mouse event `t` values are relative to this origin; the exporter subtracts `events_ms` to convert each event timestamp to a video-relative offset.
- `mic_ms: Option<u64>` - capture clock value at the first mic sample, stamped inside the `CpalMic` callback. `None` when mic was off or failed to open. *Why at first sample rather than at open:* the time between opening the device and the first callback sample is the real input latency; stamping at the first sample cancels it, so `mic_ms` is where audio content actually begins on the capture timeline.
- `system_ms: Option<u64>` - capture clock value when `SystemAudio::loopback` succeeded. `None` when system audio was off or failed. *Why at open, not first sample:* the loopback callback does not expose per-sample timing; the open time is the best available approximation.

Both `mic_ms` and `system_ms` carry `#[serde(default)]` so older `sync.json` files without these fields deserialize without error.

### Used by

- `src-tauri/src/session/record/recorder.rs` - constructed in `stop_recording` from `frame_ts`, `events_ms`, and the audio start atomics; saved via `SyncLog::save`.
- `src-tauri/src/export/pipeline/timeline.rs` - loaded via `SyncLog::load` to rebuild the frame timeline and compute audio offsets during export.

## SyncLog::save

```rust
pub fn save(&self, path: &Path) -> io::Result<()>
```

Serializes `self` to compact JSON and writes it atomically (single `fs::write` call) to `path`.

### Inputs

- `&self` - the log to serialize.
- `path: &Path` - destination file (typically `sync.json` inside the project folder). *Why `&Path` not `String`:* consistent with the `ProjectPaths` accessors and avoids redundant allocation.

### Returns

`Ok(())` on success. `Err(io::Error)` wrapping a `serde_json` error (mapped via `io::Error::new(Other, e)`) or an `fs::write` error.

### Implementation

1. `serde_json::to_vec(self)` - convert to bytes. Map any serialization error to `io::Error(Other, ...)`. *Why `to_vec` not `to_string`:* produces `Vec<u8>` directly, avoiding a UTF-8 round-trip before writing.
2. `std::fs::write(path, json)` - single syscall write. *Why not a buffered writer:* the payload is small (one entry per frame; ~8 bytes each); a single write is simpler and avoids partial-write corruption.

## SyncLog::load

```rust
pub fn load(path: &Path) -> io::Result<SyncLog>
```

Reads `sync.json` from `path` and deserializes it.

### Inputs

- `path: &Path` - source file.

### Returns

`Ok(SyncLog)` on success. `Err(io::Error)` if the file is missing, unreadable, or contains invalid JSON (mapped via `io::Error::new(Other, e)`).

### Implementation

1. `std::fs::read(path)` - read all bytes.
2. `serde_json::from_slice(&bytes)` - deserialize. Map error to `io::Error`. *Why `from_slice` not `from_str`:* avoids a copy through a `String`; the bytes are already in memory from `read`.

### Behaviors

- `round_trips_through_json`: constructs a `SyncLog` with `frames = [10, 26, 42]`, `events_ms = 5`, `mic_ms = Some(3)`, `system_ms = None`; saves to a temp file; loads back; asserts all fields match, including that `system_ms` round-trips as `None` via `serde(default)`.
