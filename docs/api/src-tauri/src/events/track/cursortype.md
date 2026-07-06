# src-tauri/src/events/track/cursortype.rs

Defines the `CursorType` enum for the nine recognized OS cursor shapes, the `CursorTrack` timeline that records when each shape was active, and persistence/lookup helpers. Used by the cursor compositor to render the correct sprite at each frame.

## CursorType

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CursorType {
    Arrow,
    IBeam,
    Hand,
    #[serde(rename = "resize_ns")]
    ResizeNs,
    #[serde(rename = "resize_ew")]
    ResizeEw,
    #[serde(rename = "resize_nwse")]
    ResizeNwse,
    #[serde(rename = "resize_nesw")]
    ResizeNesw,
    Move,
    Busy,
}
```

The nine standard OS cursor shapes tracked during recording. Corresponds to the Windows IDC constants sampled by `CursorTypeTracker`.

- `Arrow` - *default pointer; also the fallback when the shape is unrecognized (custom app cursor). Default variant.*
- `IBeam` - *text-insertion beam; shown over editable text fields.*
- `Hand` - *pointing hand; shown over links and clickable elements.*
- `ResizeNs` - *vertical resize (IDC_SIZENS); north-south window edge.*
- `ResizeEw` - *horizontal resize (IDC_SIZEWE); east-west window edge.*
- `ResizeNwse` - *diagonal resize (IDC_SIZENWSE); northwest-southeast corner.*
- `ResizeNesw` - *diagonal resize (IDC_SIZENESW); northeast-southwest corner.*
- `Move` - *four-directional move (IDC_SIZEALL); used when dragging window content.*
- `Busy` - *wait/spinner (IDC_WAIT or IDC_APPSTARTING); both map to this variant because they carry the same semantic.*

### Used by

- `src-tauri/src/events/track/cursortracker.rs` - the polling loop classifies the live cursor handle to a `CursorType` and appends changes
- `src-tauri/src/events/track/cursortype.rs` (`CursorTrack`) - stored as `(u32, CursorType)` pairs in the timeline
- `src-tauri/src/export/cursor/cursorset.rs` - selects the sprite to draw per frame based on `CursorType`
- `src-tauri/src/ai/commands.rs` / `src-tauri/src/ai/backend/timeline.rs` - the AI director reads cursor shapes to annotate the activity timeline

## CursorTrack

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CursorTrack {
    pub samples: Vec<(u32, CursorType)>,
}
```

Monotonic timeline of cursor-type changes. One entry per shape-change event (not per poll tick), so the vec is small even for long recordings.

- `samples` - *ascending list of `(t_ms, CursorType)` pairs; one entry is added whenever the shape differs from the previous poll result. Empty at the start of a session; seeded with `(0, Arrow)` if the first observed shape is unrecognized.*

### Used by

- `src-tauri/src/session/record/recorder.rs` - built from the `CursorTypeTracker` result and saved to `cursor.json`
- `src-tauri/src/export/cursor/cursorset.rs` - loaded and queried via `type_at` per frame during export
- `src-tauri/src/ai/backend/timeline.rs` - loaded for AI timeline annotation

## CursorTrack::save

```rust
pub fn save(&self, path: &Path) -> std::io::Result<()>
```

Serializes `CursorTrack` to compact JSON and writes it to `path`.

### Inputs

- `&self` - the track to persist.
- `path: &Path` - destination file (typically `<project>/cursor.json`). *Why compact rather than pretty JSON:* the cursor track can have thousands of entries in long recordings; compact saves space without impacting human readability (it is machine-read only).*

### Returns

`Ok(())` on success; `Err(io::Error)` on serialization or write failure.

### Behaviors

- `round_trip_save_load` - a three-entry track serializes and deserializes without data loss.

## CursorTrack::load

```rust
pub fn load(path: &Path) -> Self
```

Reads and deserializes `cursor.json`. Returns an empty `CursorTrack` on any error.

### Inputs

- `path: &Path` - file to read. *Why empty on error rather than `Result`:* a missing cursor track (e.g. older recording before this feature was added) should degrade gracefully to all-Arrow rendering, not abort the export.*

### Returns

`CursorTrack` - populated if the file exists and is valid; empty (`samples = []`) otherwise.

### Behaviors

- `load_missing_file_returns_empty` - a nonexistent path returns an empty track.

## CursorTrack::type_at

```rust
pub fn type_at(&self, t_ms: u32) -> CursorType
```

Returns the cursor shape active at `t_ms` using a binary search.

### Inputs

- `t_ms: u32` - the playback time to query, in recording-clock milliseconds. *Why the recording clock:* `CursorTrack` timestamps come from the same `Instant` epoch as the mouse event log.*

### Returns

The `CursorType` of the last sample whose timestamp is <= `t_ms`. Returns `Arrow` if the track is empty or all samples start after `t_ms`.

### Implementation

1. `partition_point(|&(t, _)| t <= t_ms)` gives the index of the first entry strictly after `t_ms`. O(log n).
2. If `idx == 0`, no entry precedes `t_ms`; return `Arrow`.
3. Otherwise return `samples[idx - 1].1`. *Why `partition_point` rather than a manual binary search:* it is the idiomatic Rust sorted-slice predicate search, guaranteed correct for any length including empty.*

### Behaviors

- `type_at_empty_returns_arrow` - empty track returns `Arrow` at any timestamp.
- `type_at_lookups` - a three-entry track (`Arrow@0`, `IBeam@100`, `Hand@250`) returns the correct shape at t=50, 100, 200, and 9999.
- `type_at_before_first_sample_returns_arrow` - a track whose first entry is at t=50 returns `Arrow` at t=10.
- `all_variants_serde_roundtrip` - every variant serializes and deserializes correctly.
- `expected_serde_names` - `ResizeNs` -> `"resize_ns"`, `ResizeEw` -> `"resize_ew"`, etc.
