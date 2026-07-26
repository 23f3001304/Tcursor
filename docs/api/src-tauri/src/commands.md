# src-tauri/src/commands.rs

Miscellaneous Tauri commands that do not belong to a specific subsystem. Covers display enumeration, audio-device listing, webcam blob persistence, export triggering, and settings load/save. All commands run on the Tauri thread pool; none hold long-lived state.

## DisplayInfo

```rust
#[derive(Serialize)]
pub struct DisplayInfo { pub id: u32, pub label: String }
```

Serializable descriptor for one display, returned by `list_displays`.

- `id: u32` - numeric display identifier. *Why u32:* sufficient for current use; only the primary display (id 0) is returned today.
- `label: String` - human-readable name shown in the frontend device picker.

### Used by

- `src-tauri/src/commands.rs` - constructed and returned by `list_displays`.

## AudioInfo

```rust
#[derive(Serialize)]
pub struct AudioInfo { pub id: String, pub label: String }
```

Serializable descriptor for one audio input device, returned by `list_audio_inputs`.

- `id: String` - cpal device name, passed back to `start_recording` as `mic_id`. *Why the device name as id:* cpal identifies devices by name; using name as id avoids a separate lookup table.
- `label: String` - same as `id` currently (cpal does not expose a separate display name). *Why a separate field:* allows the frontend to display a friendlier label if a mapping is added later, without changing the wire format.

### Used by

- `src-tauri/src/commands.rs` - constructed in `list_audio_inputs`.

## list_displays

```rust
#[tauri::command]
pub fn list_displays() -> Vec<DisplayInfo>
```

Returns the list of available display captures.

### Inputs

None.

### Returns

A `Vec<DisplayInfo>` always containing one entry: `{ id: 0, label: "Primary Display" }`. *Why hardcoded:* TCursor currently captures the primary display only; multi-monitor support would expand this list.

### Implementation

Returns a literal `vec![DisplayInfo { id: 0, label: "Primary Display".into() }]`. No OS query is performed.

## list_audio_inputs

```rust
#[tauri::command]
pub fn list_audio_inputs() -> Vec<AudioInfo>
```

Enumerates available microphone inputs via cpal.

### Inputs

None.

### Returns

A `Vec<AudioInfo>` of all cpal input devices on the default host, or an empty vec if cpal cannot enumerate them.

### Implementation

1. Obtain `cpal::default_host()`.
2. Call `host.input_devices()`. On error, return `unwrap_or_default()` (empty vec). *Why not propagate the error:* a missing cpal host is non-fatal; the frontend falls back to showing no mic options.
3. For each device, call `d.name().ok()?` - silently skip devices with unparseable names. Construct `AudioInfo { id: name.clone(), label: name }`.

## save_webcam

```rust
#[tauri::command]
pub fn save_webcam(folder: String, bytes: Vec<u8>) -> Result<(), String>
```

Writes a raw webcam blob (webm) sent from the frontend to disk.

### Inputs

- `folder: String` - project folder path, as returned by `stop_recording`. *Why String not Path:* Tauri commands serialize all arguments from JSON; `String` is the natural wire type.
- `bytes: Vec<u8>` - the raw `.webm` bytes captured by the browser MediaRecorder API. *Why Vec<u8>:* the frontend sends the blob as a byte array over the Tauri IPC bridge; no encoding conversion is needed.

### Returns

`Ok(())` on success. `Err(String)` with the `io::Error` message on write failure.

### Implementation

1. Construct `path = Path::new(&folder).join("webcam.webm")`. *Why a fixed filename:* matches `ProjectPaths::webcam()`, so the exporter can locate the file via `ProjectPaths` without re-querying the frontend.
2. Call `std::fs::write(path, bytes)`. Map error to `String`.

## export_project

```rust
#[tauri::command]
pub fn export_project(folder: String, settings: crate::export::settings::ExportSettings, app: tauri::AppHandle) -> Result<(), String>
```

Kicks off the async export pipeline for a completed recording.

### Inputs

- `folder: String` - project folder path returned by `stop_recording`. *Why String:* passed through to `run_export` which resolves it into `ProjectPaths`.
- `settings: ExportSettings` (`export::settings::ExportSettings`) - the user's chosen resolution/fps/quality/format, collected by the frontend `ExportDialog`. *Why a full struct rather than individual args:* every field is optional-in-spirit (has a sensible default via `#[serde(default)]`), and threading one struct through `run_export` -> `exporter::export` avoids repeating four parameters at every layer.
- `app: tauri::AppHandle` - used by `run_export` to emit progress events back to the frontend via Tauri's event system.

### Returns

`Ok(())` immediately - the export runs in the background. `run_export` emits events for progress and completion.

### Implementation

1. Call `crate::export::pipeline::run::run_export(app, folder, settings)`. *Why not await:* export is long-running (seconds to minutes); `run_export` spawns its own task and emits events asynchronously.
2. Return `Ok(())`.

## get_settings

```rust
#[tauri::command]
pub fn get_settings() -> crate::settings::model::Settings
```

Loads and returns the current persisted settings.

### Inputs

None.

### Returns

The current `Settings` value. `settings::store::load` never fails (returns defaults on any error), so no `Result` is needed.

### Implementation

Delegates to `crate::settings::store::load()` and returns the result directly.

## set_settings

```rust
#[tauri::command]
pub fn set_settings(settings: crate::settings::model::Settings) -> Result<(), String>
```

Persists a new settings value.

### Inputs

- `settings: crate::settings::model::Settings` - the full settings struct sent from the frontend. *Why the entire struct rather than individual fields:* avoids partial-update races; the frontend owns the canonical copy and sends it atomically.

### Returns

`Ok(())` on success. `Err(String)` if the write fails (e.g. disk full, permission denied).

### Implementation

1. Call `crate::settings::store::save(&settings)`. Map `io::Error` to `String` via `.map_err(|e| e.to_string())`.

## set_capturable

```rust
#[tauri::command]
pub fn set_capturable(app: tauri::AppHandle, capturable: bool) -> bool
```

Toggles whether the app window appears in screen capture / screenshots.

### Inputs

- `app: tauri::AppHandle` - injected by Tauri; used to resolve the `"main"` webview window so the affinity is set on the exact `HWND` the startup exclusion used. *Why the app handle, not the calling window:* guarantees no window-handle mismatch with the startup exclusion.
- `capturable: bool` - true to make the window visible to capture (the editor), false to hide it (the HUD). *Why:* the HUD is capture-excluded so it never shows in recordings; the editor opts back in so it can be screenshotted.

### Returns

`bool` - the result of `set_capture_exclusion` (true on success), or `false` if the window/`HWND` is unavailable or on non-Windows.

### Implementation

1. (Windows) `app.get_webview_window("main")` -> `win.hwnd()` -> `win::capture_exclusion::set_capture_exclusion(hwnd, !capturable)` (capturable inverts exclude).
2. (non-Windows) discard the args and return `false`.
