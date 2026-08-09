# src-tauri/src/settings/store.rs

Thin persistence layer for `Settings`: computes the canonical config-file path, loads settings from disk (returning defaults on any error), and saves settings as pretty-printed JSON. The module contains no business logic; all policy lives in the `Settings` struct and its `Default` impl.

It covers **two different files** that must not be confused:

- `config_path()` -> `<config-dir>/TCursor/config.json`, the app's LIVE settings, rewritten every time the user changes a preference (`load`/`save`).
- `paths.settings()` -> `<recording folder>/settings.json`, the **record-time snapshot** (`record_snapshot`), written once by `start_recording` and never again. It is the only durable record of how a given video was actually captured, which matters for any property the editor can no longer infer - see `os_cursor_in_video`.

## record_snapshot

```rust
pub fn record_snapshot(paths: &ProjectPaths) -> Settings
```

The settings `start_recording` froze into the recording folder. Defaults on a missing or corrupt file, exactly like `load`.

*Why it is not the same as the doc's `settings`:* `edit.json`'s copy starts as this snapshot but is then edited freely, so it describes what the user wants NOW, not how the recording was captured. Anything record-time must read this instead.

*Why this is safe to rely on:* the folder's `settings.json` has exactly one writer in the codebase (`session/record/recorder.rs`, at record start) and two readers (`edit::seed::build_default` and this module). The editor never writes back to it - `settings::save` targets `config_path()`, a completely different file.

### Used by

- `src-tauri/src/edit/seed.rs` (`build_default`) - seeds a new `EditDoc` from the settings the recording was made with.
- `src-tauri/src/settings/store.rs` (`os_cursor_in_video`) - below.

## os_cursor_in_video

```rust
pub fn os_cursor_in_video(paths: &ProjectPaths) -> bool
```

Whether this recording's video has the OS cursor baked into its pixels: `record_snapshot(paths).cursor.style.captures_os_cursor()`.

*Why it must be derived, not stored:* `captures_os_cursor` is a capture-time decision (it tells WGC whether to composite the cursor), but the editor lets the user change `cursor.style` afterwards. Picking `System` on a recording made in `Enhanced` used to turn the synthetic cursor off while there was no real one in the video either - so the cursor disappeared entirely. Deriving the answer from the snapshot recovers the truth for every existing recording, with no manifest field and no migration.

*Why a missing snapshot answers `true`:* it fails closed. `true` means "draw no synthetic cursor", which is the pre-existing behavior; guessing `false` could paint a second cursor on top of a real one.

### Behaviors

- `os_cursor_in_video_is_derived_from_the_record_time_snapshot` - a snapshot written with `System` reports `true`; `Enhanced` and `Hidden` report `false`.
- `a_missing_snapshot_reports_a_baked_cursor` - a folder with no `settings.json` reports `true`.

## config_path

```rust
pub fn config_path() -> PathBuf
```

Returns the path `<platform-config-dir>/TCursor/config.json`. The file does not need to exist for this function to succeed.

### Inputs

None. The function reads the process environment internally.

### Returns

`PathBuf` constructed from `dirs_next::config_dir()` joined with `"TCursor/config.json"`. Falls back to `std::env::temp_dir()` if `config_dir()` returns `None`. On Windows the typical result is `%APPDATA%\TCursor\config.json`; on macOS it would be `~/Library/Application Support/TCursor/config.json`.

### Implementation

1. Call `dirs_next::config_dir()` and unwrap with `unwrap_or_else(std::env::temp_dir)`. *Why `temp_dir` as fallback:* ensures the path is always valid and writable, avoiding a panic in constrained environments (e.g. sandboxed CI).
2. Call `.join("TCursor").join("config.json")`.

### Behaviors

- `config_path_is_under_tcursor` - asserts the result ends in `"config.json"` and contains `"TCursor"` in its string representation.

### Used by

- `src-tauri/src/settings/store.rs` (`load`, `save`) - both read this path internally

## load

```rust
pub fn load() -> Settings
```

Reads and deserialises the config file, returning `Settings::default()` on any failure.

### Inputs

None. Reads from the path returned by `config_path()`.

### Returns

`Settings` - either the fully deserialised user config or `Settings::default()` if the file is absent, unreadable, or contains malformed JSON. Partial JSON with missing fields deserialises successfully because every `Settings` field carries `#[serde(default)]`; only truly malformed bytes trigger the fallback.

### Implementation

1. `std::fs::read(config_path())` - returns a `Result<Vec<u8>>`. Convert to `Option` via `.ok()`.
2. `.and_then(|b| serde_json::from_slice(&b).ok())` - deserialise; convert errors to `None`.
3. `.unwrap_or_default()` - return `Settings::default()` if either step yielded `None`.

*Why chained `ok()` rather than `?`:* the caller has no error channel to propagate to (`run` just needs settings); silently falling back to defaults is the correct behavior for a missing or corrupt config.

### Used by

- `src-tauri/src/commands.rs` (`get_settings`) - serves the current settings over IPC to the frontend
- `src-tauri/src/session/record/recorder.rs` - called at recording start to snapshot all settings for the session duration

## save

```rust
pub fn save(s: &Settings) -> std::io::Result<()>
```

Serialises `s` to pretty-printed JSON and writes it to `config_path()`, creating parent directories if needed.

### Inputs

- `s: &Settings` - the settings snapshot to persist. *Why a reference:* the caller retains ownership; `save` is a write-through that does not consume or transform the value.

### Returns

`std::io::Result<()>` - `Ok(())` on success. Propagates `io::Error` from `create_dir_all` or `fs::write`. Wraps any `serde_json` serialisation error as `ErrorKind::Other`. *Why `io::Result` rather than a custom error:* the only caller (`commands::set_settings`) maps the error to a `String` for the IPC response; `io::Error` provides a readable message without an additional error type.

### Implementation

1. Compute `path = config_path()`.
2. `fs::create_dir_all(path.parent())` - ensures the `TCursor` directory exists before writing. Returns early on error. *Why `parent()` rather than a hardcoded dir:* keeps the directory in sync with `config_path()` automatically.
3. `serde_json::to_vec_pretty(s)` - serialise. Map the serde error to `io::Error::new(ErrorKind::Other, e)` so the return type is uniform.
4. `fs::write(path, json)` - atomic on most platforms (write to a temp file then rename is handled by the OS on Linux/macOS; on Windows `write` is a direct overwrite but config files are small enough that partial-write risk is negligible).

### Used by

- `src-tauri/src/commands.rs` (`set_settings`) - called after the frontend submits updated settings over IPC
