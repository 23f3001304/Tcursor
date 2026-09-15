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

Whether this recording's video has the OS cursor baked into its pixels: `record_snapshot(paths).cursor.style.captures_os_cursor() && !CursorLayer::exists(paths)`.

**True only for a PRE-LAYER `System` recording.** Capture is now always cursor-free (`recorder.rs` passes `with_cursor: false` for every style) and the real cursor is recorded as its own layer instead, so the only recordings with a baked cursor are the ones made before that change. A cursor layer on disk is the marker that the take came after it, and it beats the style every time; without one, the record-time style is the only evidence there is, and back then `System` did mean baked pixels.

*Why it must be derived, not stored:* `captures_os_cursor` was a capture-time decision, but the editor lets the user change `cursor.style` afterwards. Picking `System` on a recording made in `Enhanced` used to turn the synthetic cursor off while there was no real one in the video either - so the cursor disappeared entirely. Deriving the answer recovers the truth for every existing recording, with no manifest field and no migration.

*Why a missing snapshot answers `true`:* it fails closed (on a project with no layer). `true` means "draw no synthetic cursor", which is the pre-existing behavior; guessing `false` could paint a second cursor on top of a real one.

### Behaviors

- `os_cursor_in_video_is_derived_from_the_record_time_snapshot` - with no layer present, a snapshot written with `System` reports `true`; `Enhanced` and `Hidden` report `false`.
- `a_cursor_layer_means_the_video_is_clean_whatever_the_style_was` - the full 2x2 matrix (System/Enhanced x layer/no-layer): only System-without-a-layer reports `true`.
- `a_missing_snapshot_reports_a_baked_cursor` - a folder with no `settings.json` and no layer reports `true`.

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

Thin wrapper: `load_from(&config_path())`.

### Used by

- `src-tauri/src/commands.rs` (`get_settings`) - serves the current settings over IPC to the frontend
- `src-tauri/src/session/record/recorder.rs` - called at recording start to snapshot all settings for the session duration

## load_from

```rust
fn load_from(path: &Path) -> Settings
```

`load`'s actual logic, taking an explicit path - split out (mirroring `EditDoc::load`) so it is unit-testable against a throwaway temp file instead of the real `config_path()`.

### Inputs

- `path: &Path` - file to read.

### Returns

`Settings` - either the fully deserialised config or `Settings::default()` if the file is absent, unreadable, or contains malformed JSON. Partial JSON with missing fields deserialises successfully because every `Settings` field carries `#[serde(default)]`; only truly malformed bytes trigger the fallback.

### Implementation

1. `std::fs::read(path)`. A missing/unreadable file -> `Settings::default()` directly (no corrupt-preservation - there is nothing to preserve).
2. On successfully-read bytes, `serde_json::from_slice`. On success, return the parsed `Settings`.
3. **On a parse failure (M3, bug-sweep-2):** call `process::proc::preserve_corrupt(path, &e)` - moves the bad bytes aside to `<path>.corrupt` and logs the parse error - THEN return `Settings::default()`. Before this fix the bytes were simply discarded (`.ok()` chained straight to `unwrap_or_default()`), so a torn write from a crash mid-`save` silently reset every hotkey/theme/spotlight/audio setting with zero recovery path; now the original bytes survive on disk next to the file, same guarantee `EditDoc::load` already gave `edit.json`.

### Behaviors

- `load_from_a_truncated_file_returns_defaults_and_preserves_the_original_at_dot_corrupt` - malformed JSON returns `Settings::default()` and the original bytes end up unmodified at `<path>.corrupt`; the original path no longer holds the bad bytes.
- `load_from_a_missing_file_returns_defaults_without_touching_disk` - a non-existent path returns defaults and creates neither the file nor a `.corrupt` sibling.

## save

```rust
pub fn save(s: &Settings) -> std::io::Result<()>
```

Thin wrapper: `save_to(&config_path(), s)`.

### Used by

- `src-tauri/src/commands.rs` (`set_settings`) - called after the frontend submits updated settings over IPC

## save_to

```rust
fn save_to(path: &Path, s: &Settings) -> std::io::Result<()>
```

`save`'s actual logic, taking an explicit path (mirroring `EditDoc::save`, split out for the same testability reason as `load_from`). Serialises `s` to pretty-printed JSON and writes it ATOMICALLY to `path`: the bytes land at a temp sibling first, then `std::fs::rename` moves them into place - the same tmp+rename pattern `EditDoc::save` uses.

### Inputs

- `path: &Path` - destination file.
- `s: &Settings` - the settings snapshot to persist. *Why a reference:* the caller retains ownership; `save_to` is a write-through that does not consume or transform the value.

### Returns

`std::io::Result<()>` - `Ok(())` on success. Propagates `io::Error` from `create_dir_all`, the temp-file write, or the rename. Wraps any `serde_json` serialisation error as `ErrorKind::Other`. *Why `io::Result` rather than a custom error:* the only caller (`commands::set_settings`) maps the error to a `String` for the IPC response; `io::Error` provides a readable message without an additional error type.

### Implementation

1. `fs::create_dir_all(path.parent())` - ensures the `TCursor` directory exists before writing. Returns early on error. *Why `parent()` rather than a hardcoded dir:* keeps the directory in sync with `config_path()` automatically.
2. `serde_json::to_vec_pretty(s)` - serialise. Map the serde error to `io::Error::new(ErrorKind::Other, e)` so the return type is uniform.
3. **(M3, bug-sweep-2)** `process::proc::tmp_sibling(path)` - a unique temp path in the same directory. Write the JSON there; on write failure, remove the temp and return the error (nothing at `path` is touched).
4. `fs::rename(&tmp, path)` - atomically replaces `path`. *Why this replaced the old direct `fs::write(path, json)`:* `fs::write` truncates then writes IN PLACE - a crash or forced quit between those two steps left `config.json` truncated, which `load_from` would then read back as "corrupt" and silently reset to defaults, discarding every hotkey/theme/spotlight/audio setting with no recovery path. Settings panels write on every slider tick with no debounce (`SettingsPanel.tsx`, `Preferences.tsx`), which is the maximum-exposure pattern for an in-place write.

### Behaviors

- `save_to_then_load_from_round_trips_and_leaves_no_tmp_sibling` - a saved `Settings` round-trips byte-for-byte through `load_from`, and the directory contains only `config.json` afterward - no leftover `.part-*` temp file.
- `save_to_overwrites_an_existing_file` - a second `save_to` call fully replaces the first snapshot.
