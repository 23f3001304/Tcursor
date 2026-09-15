# src-tauri/src/actions/model.rs

Data model for hotkey-driven capture actions: enumerated action kinds, individual timestamped events, and a JSON-serializable log with load/save helpers. Pure data - no I/O logic beyond serialization, no threading, no side effects on the recording pipeline.

## LayoutId

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutId { Screen, Camera, Presenter, ScreenOnly, CameraOnly }
```

Identifies one of five layout presets that a layout hotkey selects at recording time.

- `Screen` - full-screen layout. *Why:* the most common preset; maps to the default `layout_screen` hotkey.
- `Camera` - camera-only layout. *Why:* presenter wants to show themselves full-frame.
- `Presenter` - picture-in-picture or side-by-side presenter layout. *Why:* the most complex preset; stored as a discriminant so the frontend can reconstruct the panel arrangement.
- `ScreenOnly` - screen without camera overlay. *Why:* distinct from `Screen` when a camera is connected but the presenter wants to hide it.
- `CameraOnly` - camera without screen. *Why:* for reaction or commentary segments.

Serializes to snake_case JSON strings via `#[serde(rename_all = "snake_case")]` (e.g. `"screen_only"`). *Why snake_case:* consistent with the rest of the JSON event logs so the frontend and AI prompt receive uniform identifiers.

### Used by

- `src-tauri/src/actions/matcher.rs` - `arming_from_settings` maps each `HotkeySettings` layout field to a `SetLayout(LayoutId)` arm.
- `src-tauri/src/export/fx/caption.rs` - `caption_at` matches `SetLayout(id)` to look up the hotkey string to display as an on-screen caption.
- `src-tauri/src/export/render/fromedit.rs` - `layout_segs_from_doc` constructs `ActionEvent` values carrying `LayoutId` to feed the export pipeline.

## ActionKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    SetLayout(LayoutId),
    ZoomHoldStart, ZoomHoldEnd,
    SpotlightHoldStart, SpotlightHoldEnd,
    VideoFxHoldStart, VideoFxHoldEnd,
}
```

Discriminates the type of action captured at a point in time.

- `SetLayout(LayoutId)` - emitted once on key-down; carries the target preset. Serializes as `{"set_layout": "camera"}`. *Why a newtype variant:* keeps the layout id co-located with the action in the JSON log without a separate field.
- `ZoomHoldStart` / `ZoomHoldEnd` - bracket a manual zoom-hold segment; `Start` fires on key-down, `End` on key-up. *Why paired:* the exporter needs the exact interval to synthesize a `ZoomRegion` without heuristics.
- `SpotlightHoldStart` / `SpotlightHoldEnd` - same pairing for the spotlight effect. *Why separate from zoom:* spotlight and zoom are distinct visual effects with different rendering paths.
- `VideoFxHoldStart` / `VideoFxHoldEnd` - same pairing for a generic video FX trigger. *Why:* extensibility without changing the data model.

Serializes to snake_case (e.g. `"zoom_hold_start"`). `Copy` because events are stored in a `Vec` and cloned/compared frequently.

### Used by

- `src-tauri/src/actions/matcher.rs` - `Arm.on_down` and `Arm.on_up` carry `ActionKind` values; `ActionMatcher::on_key` emits them.
- `src-tauri/src/platform/windows/input/hotkeys.rs` - `Win32Hotkeys` accumulates `ActionEvent` values containing `ActionKind`.
- `src-tauri/src/export/fx/caption.rs` - matched in `caption_at` to determine which hotkey label to render.
- `src-tauri/src/export/fx/hold.rs`, `src-tauri/src/export/fx/spotlight.rs`, `src-tauri/src/export/camera/manual.rs` - consume `ActionKind` variants to drive hold, spotlight, and manual zoom rendering.
- `src-tauri/src/ai/backend/timeline.rs` - filters `SetLayout` variants into the AI transcript.

## ActionEvent

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEvent { pub t: u32, pub kind: ActionKind }
```

A single timestamped hotkey action.

- `t` - milliseconds elapsed since the start of the recording session (session-relative, not Unix time). *Why `u32`:* a 32-bit ms counter can represent ~49 days; sessions are always shorter.
- `kind` - which action occurred. *Why inline rather than separate timestamp + kind fields:* keeping them together simplifies sorting, filtering, and serialization - the whole event fits in one serde round-trip.

### Used by

- `src-tauri/src/platform/windows/input/hotkeys.rs` - produced by `Win32Hotkeys` and returned from `stop()`.
- `src-tauri/src/actions/model.rs` - stored in `ActionLog.actions`.
- `src-tauri/src/ai/backend/timeline.rs` - consumed by `serialize` to emit layout-change lines in the AI transcript.
- `src-tauri/src/export/fx/caption.rs` - sliced and searched in `caption_at`.
- `src-tauri/src/export/render/fromedit.rs` - constructed by `layout_segs_from_doc` to feed the export pipeline.

## ActionLog

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ActionLog {
    #[serde(default)]
    pub actions: Vec<ActionEvent>,
}
```

Container for all `ActionEvent` values from one recording session, with JSON persistence.

- `actions` - the ordered list of events, ascending in `t`. `#[serde(default)]` means a JSON object missing the `"actions"` key deserializes to an empty vec rather than an error. *Why:* older session files created before a hotkey feature was added should still load without failure.

### Used by

- `src-tauri/src/session/record/recorder_threads.rs` - constructs `ActionLog { actions }` and calls `save(actions_path)` when a recording ends.
- `src-tauri/src/ai/run.rs` - `propose` calls `ActionLog::load` to read the session's hotkey log for the AI transcript.
- `src-tauri/src/export/pipeline/exporter.rs` - loads `ActionLog` to feed the export pipeline.

## ActionLog::save

```rust
pub fn save(&self, path: &Path) -> io::Result<()>
```

Persists the log to disk as compact JSON.

### Inputs

- `&self` - the log to serialize. *Why by reference:* save is non-consuming; the log remains usable after saving.
- `path: &Path` - destination file path. *Why:* callers pass `paths.actions()` so the path is determined by the session directory, not hardcoded here.

### Implementation

1. `serde_json::to_vec(self)` - serializes to a byte vec. `to_vec` rather than `to_string` avoids a UTF-8 validation step on output. Serialization errors (unexpected in practice) are wrapped as `io::ErrorKind::Other`.
2. `std::fs::write(path, json)` - writes atomically at the OS level (single `write` syscall on most platforms). *Why not a buffered writer:* the log is small; a single write avoids partial-file states on crash.

### Returns

`io::Result<()>` - `Ok` on success; `Err` on serialization failure or I/O error.

## ActionLog::load

```rust
pub fn load(path: &Path) -> io::Result<ActionLog>
```

Reads and deserializes the log from disk.

### Inputs

- `path: &Path` - source file path. *Why:* callers pass `paths.actions()`; the path is session-scoped so no global state is accessed.

### Implementation

1. `std::fs::read(path)` - reads the entire file into a `Vec<u8>`. Returns `Err` if the file is missing or unreadable.
2. `serde_json::from_slice` - deserializes without an intermediate `String` allocation. Returns `Err` (wrapped as `io::ErrorKind::Other`) on invalid JSON or unknown fields.

### Returns

`io::Result<ActionLog>`. Callers that treat a missing file as "no hotkey actions recorded" typically chain `.unwrap_or_default()`.

### Behaviors

- `round_trips_actions_json` - a log with `SetLayout(Camera)`, `ZoomHoldStart`, and `ZoomHoldEnd` serializes to JSON containing `"set_layout":"camera"` and `"zoom_hold_start"`, then deserializes back with all three events intact and in order.
