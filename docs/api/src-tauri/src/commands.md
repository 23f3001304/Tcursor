# src-tauri/src/commands.rs

Miscellaneous Tauri commands that do not belong to a specific subsystem. Covers display enumeration, audio-device listing, webcam blob persistence, export triggering, and settings load/save. All commands run on the Tauri thread pool; none hold long-lived state.

## DisplayInfo

```rust
#[derive(Serialize)]
pub struct DisplayInfo {
    pub id: String,
    pub label: String,
    pub kind: String,
}
```

Serializable descriptor for one capture target, returned by `list_displays`.

- `id: String` - the target's wire id, the string form of `ports::capture::TargetId` (`display:N` or `window:0x...`), which `start_recording`, `get_target_bounds` and `switch_display` parse back through `TargetId::from_arg`; an unparseable id resolves to the primary display, so the HUD can never pick a target the recorder cannot open.
- `label: String` - human-readable name shown in the frontend device picker. For a display the size and primary flag ride on the end as `(WxH, Primary)`, `(Primary)` or `(WxH)`; the HUD's `parseTarget` splits them back out (the map in `TargetSheet` draws each monitor to scale from the size).
- `kind: String` - `"display"` or `"window"`, the `TargetKind` the picker groups by.

### Used by

- `src-tauri/src/commands.rs` - constructed and returned by `list_displays`, from the Windows capture adapter's enumeration (`platform/windows/capture/target.md`).

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
pub fn list_displays(platform: tauri::State<'_, Arc<Platform>>) -> Vec<DisplayInfo>
```

Everything the user can record, as the frontend's target picker wants it: every monitor, then every ordinary application window.

### Inputs

- `platform: tauri::State<'_, Arc<Platform>>` - the adapter bundle, injected by Tauri, so the JS call is unchanged.

### Returns

A `Vec<DisplayInfo>` whose `id` is a `ports::capture::TargetId` rendered back to a string (`display:N`, `window:0x<hex>`) and whose `kind` is `"display"` or `"window"`. Never empty on Windows: when no monitor enumerates at all, a single synthetic `display:0` "Primary Display" entry stands in so the picker has something to show, and the windows are appended after it either way.

### Implementation

`platform.capture.list_targets()`, mapped down from `CaptureTarget` to `DisplayInfo`. The enumeration itself - `Monitor::enumerate` for the displays, `EnumWindows` + `GetWindowLongW` + `GetWindowTextW` for the windows, and the filter that drops tool windows, untitled windows, `Program Manager`, `Settings`, `TCursor` itself and the `MSCTFIME` IME host - lives in `platform/windows/capture/target.rs`, behind the adapter. It used to live here, with the `Monitor::enumerate` call UNGATED, which is one of the two reasons this file did not compile off Windows (`docs/cross-platform-architecture.md`, section 2.4).

There is no `#[cfg]` left in this command and no non-Windows fallback list: Batch D deleted both. `Platform` is whatever `platform::current()` built, so the answer comes from an adapter on every target, and a target with no adapter never gets this far.

*Why the command and not `Win32Capture::list_targets` owns the `DisplayInfo` conversion:* the port speaks `CaptureTarget`, the frontend speaks `DisplayInfo`, and the conversion belongs at the Tauri boundary. Before Batch C1 it ran the other way - the adapter re-parsed the id string the command had just formatted - which is exactly the round trip a typed `TargetId` is meant to delete.

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

## append_webcam

```rust
#[tauri::command]
pub fn append_webcam(folder: String, bytes: Vec<u8>, segment: Option<u32>) -> Result<(), String>
```

Appends one `MediaRecorder` chunk to this take's webcam file DURING recording, so Stop has almost nothing left to write instead of one O(clip-length) blob. This is what the HUD uses; the one-shot `save_webcam` it superseded (a registered command with a wrapper and no caller) was removed in the cleanup of 2026-09-15.

### Inputs

- `folder: String` - the project folder `start_recording` returned. It is freshly created per recording, so the first append creates the file.
- `bytes: Vec<u8>` - one chunk (a 1s `MediaRecorder` timeslice).
- `segment: Option<u32>` - which webcam file the chunk belongs to, resolved by `session::record::webcam_segments::webcam_segment_name`: `None`/`Some(1)` is `webcam.webm`, `Some(n)` is `webcam_<n>.webm`. *Why the option exists:* a mid-take camera switch needs a SECOND `MediaRecorder` (one cannot change its stream) and therefore a second file; `preprocess` merges them back into one `webcam.webm` before the editor opens. `None` keeps a frontend that predates switching working unchanged.

### Returns

`Ok(())` on success. `Err(String)` with the `io::Error` message if the file cannot be opened or the write fails.

### Implementation

1. `webcam_segment_name(segment)` -> the file name; join it onto `folder`.
2. Open with `create(true).append(true)` and `write_all` the chunk. Appending (not rewriting) is the whole point: the file grows a second at a time and is complete as soon as the last chunk lands.

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
pub fn set_capturable(app: tauri::AppHandle, capturable: bool, platform: tauri::State<'_, Arc<Platform>>) -> bool
```

Toggles whether the app window appears in screen capture / screenshots.

### Inputs

- `app: tauri::AppHandle` - injected by Tauri; used to resolve the `"main"` webview window so the affinity is set on the exact `HWND` the startup exclusion used. *Why the app handle, not the calling window:* guarantees no window-handle mismatch with the startup exclusion.
- `capturable: bool` - true to make the window visible to capture (the editor), false to hide it (the HUD). *Why:* the HUD is capture-excluded so it never shows in recordings; the editor opts back in so it can be screenshotted.
- `platform: tauri::State<'_, Arc<Platform>>` - the adapter bundle; `platform.system` is what sets the affinity and reads it back. Injected, so the JS call is unchanged.

### Returns

`bool` - whether the window NOW HAS the requested affinity, read back by the adapter rather than assumed, or `false` if the window or its handle is unavailable and on any platform with no such facility.

### Implementation

1. `app.get_webview_window("main")` -> `shell::window::handle(&win)` -> `platform.system.exclude_from_capture(handle, !capturable)` (capturable inverts exclude).
2. `None` at either step returns `false`. There is no `#[cfg]` left in this command: Batch C3 pushed the handle resolution down into `shell::window::handle`, taking the window as a parameter so Studio's launcher window can use the same path, and Batch D replaced the transitional `platform::exclude_from_capture` free function with the port.
