# src-tauri/src/events/model.rs

Core data types for the mouse-event log: button identifiers, event kinds, individual events, screen geometry, and the top-level `EventLog` with save/load helpers. These types are the lingua franca of the events subsystem - every component that records, filters, or consumes mouse activity uses them.

## Button

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Button { Left, Right, Middle }
```

Which mouse button was pressed or released.

- `Left` - *primary button; the dominant click signal for auto-zoom and click-fx.*
- `Right` - *secondary button; tracked but not currently used by auto-zoom.*
- `Middle` - *middle button; included for completeness; not used by any current consumer.*

### Used by

- `src-tauri/src/events/model.rs` (`MouseEvent`) - optional field on `Down`/`Up` events
- `src-tauri/src/platform/windows/input/pointer.rs` - Windows hook maps `WM_LBUTTONDOWN` etc. to `Button` variants
- `src-tauri/src/events/collector.rs` - forwarded to `MouseEvent` without inspection

## EventKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind { Move, Down, Up }
```

The type of mouse action.

- `Move` - *cursor position update; subject to throttling and deduplication in `EventCollector`.*
- `Down` - *button press; used by `autozoom::generate` as the primary trigger signal; passed through `EventCollector` unfiltered.*
- `Up` - *button release; used by `clickfx` for click-animation timing; passed through unfiltered.*

### Used by

- `src-tauri/src/events/collector.rs` - branches on `Move` to apply throttle/dedup filters
- `src-tauri/src/export/camera/autozoom.rs` - filters to `Down` events only
- `src-tauri/src/export/fx/fx_state.rs` / `src-tauri/src/export/fx/clickfx.rs` - inspects `Down` and `Up` for click-fx state
- `src-tauri/src/export/camera/manual.rs` - reads `Down` events near manual zoom actions
- `src-tauri/src/ai/backend/timeline.rs` - classifies events for AI timeline annotation

## MouseEvent

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct MouseEvent {
    pub t: u32,
    pub kind: EventKind,
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<Button>,
}
```

One recorded mouse event. Compact and `Copy` so slices can be passed cheaply everywhere.

- `t` - *recording-clock timestamp in milliseconds from session start; ascending in the stored log.*
- `kind` - *the action type; drives how consumers interpret `x`, `y`, and `button`.*
- `x`, `y` - *screen pixel coordinates (signed; monitors left of the primary have negative x). Stored in screen space; converted to frame space by `coordmap::to_frame` during export.*
- `button` - *`Some` only for `Down`/`Up`; `None` for `Move`. Serialized with `skip_serializing_if = "Option::is_none"` to keep the JSON compact for the common move case.*

### Used by

- `src-tauri/src/export/camera/autozoom.rs` - the full event slice is the primary input to `generate`
- `src-tauri/src/export/cursor/mod.rs` - positions are interpolated to place the cursor sprite per frame
- `src-tauri/src/export/fx/clickfx.rs` - `Down`/`Up` pairs drive click-ripple animations
- `src-tauri/src/export/camera/manual.rs` - used to find the nearest click to a manual zoom action
- `src-tauri/src/ai/backend/timeline.rs` - events are annotated onto the AI activity timeline

## ScreenInfo

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenInfo { pub w: u32, pub h: u32, pub origin_x: i32, pub origin_y: i32 }
```

Geometry of the captured screen (or monitor) at recording time.

- `w`, `h` - *pixel dimensions of the capture region. Used as the denominator in coordinate normalization.*
- `origin_x`, `origin_y` - *top-left corner of the capture region in global screen space (signed; non-zero on multi-monitor setups where the capture monitor is not the primary). Used by `coordmap::to_frame` to convert screen-space mouse coordinates into capture-local frame coordinates.*

### Used by

- `src-tauri/src/export/camera/autozoom.rs` - `generate` passes `screen` to `coordmap::to_frame` for each click anchor
- `src-tauri/src/export/coordmap.rs` - the primary consumer for coordinate conversion
- `src-tauri/src/export/cursor/mod.rs` - normalizes mouse positions for cursor placement
- `src-tauri/src/ai/backend/timeline.rs` - used to normalize coordinates for the AI director

## EventLog

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventLog {
    pub started_unix_ms: u64,
    pub screen: ScreenInfo,
    #[serde(default)]
    pub events: Vec<MouseEvent>,
}
```

Root of `events.json`: the full mouse recording for one session.

- `started_unix_ms` - *Unix epoch milliseconds when recording began; used to correlate the event timeline with external wall-clock events (e.g. audio timestamps). Not used by the export pipeline itself.*
- `screen` - *capture geometry frozen at session start; consumers use it throughout the export without re-reading settings.*
- `events` - *the filtered, deduplicated event list produced by `EventCollector::take`. `#[serde(default)]` allows loading an older `events.json` that was written before this field existed.*

### Used by

- `src-tauri/src/session/record/recorder.rs` / `src-tauri/src/session/record/recorder_threads.rs` - constructed during recording and saved via `save`
- `src-tauri/src/edit/seed.rs` - loaded to build the initial `EditDoc`
- `src-tauri/src/export/pipeline/exporter.rs` - the export pipeline's primary event input
- `src-tauri/src/ai/commands.rs` / `src-tauri/src/ai/backend/timeline.rs` - loaded for AI timeline annotation

## EventLog::save

```rust
pub fn save(&self, path: &Path) -> io::Result<()>
```

Serializes `EventLog` to compact JSON and writes it to `path`.

### Inputs

- `path: &Path` - destination file (typically `<project>/events.json`). *Why compact not pretty:* large recordings produce tens of thousands of events; compact JSON is meaningfully smaller.*

### Returns

`Ok(())` on success; `Err(io::Error)` on serialization or write failure. Serialization errors are wrapped as `io::ErrorKind::Other`.

## EventLog::load

```rust
pub fn load(path: &Path) -> io::Result<EventLog>
```

Reads and deserializes `events.json`.

### Inputs

- `path: &Path` - file to read. *Why `io::Result` rather than `Option`:* unlike `CursorTrack`, a missing event log is a genuine error (the project is incomplete); callers handle it explicitly.*

### Returns

`Ok(EventLog)` on success; `Err(io::Error)` if the file is missing or the JSON is invalid.

### Behaviors

- `round_trips_through_json` - `Move` events serialize `kind: "move"`, `Down` events serialize `button: "left"`; a two-event log round-trips with correct field values.
- `omits_button_for_moves` - a `Move` event JSON string does not contain the `"button"` key.
