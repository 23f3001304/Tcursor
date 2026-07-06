# src-tauri/src/events/track/typing.rs

Persists a privacy-safe keystroke timeline (timestamps only, no key identities) used by the smart-zoom-hold heuristic to detect typing activity. Key identities are never recorded or stored - the log is a bare list of millisecond timestamps.

## TypingLog

```rust
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct TypingLog { pub ms: Vec<u32> }
```

The complete keystroke timeline for one recording session.

- `ms` - *ascending list of keystroke timestamps in recording-clock milliseconds. One entry per keypress event, regardless of which key. Used by `autozoom::generate` to extend a zoom hold when the user keeps typing after clicking.*

### Used by

- `src-tauri/src/session/record/recorder_threads.rs` - the keyboard hook appends to a `Vec<u32>` and calls `save` at session end
- `src-tauri/src/edit/seed.rs` - `build_default` loads the typing log and passes `log.ms` to `autozoom::generate` and `manual::from_actions`
- `src-tauri/src/export/pipeline/exporter.rs` - the export pipeline loads it for the same purpose

## TypingLog::save

```rust
pub fn save(&self, path: &Path) -> std::io::Result<()>
```

Serializes the log to compact JSON and writes it to `path`.

### Inputs

- `&self` - the log to persist.
- `path: &Path` - destination file (typically `<project>/typing.json`). *Why compact JSON:* the list is machine-read only; compact serialization is smaller for long sessions with many keystrokes.*

### Returns

`Ok(())` on success; `Err(io::Error)` on serialization or write failure.

### Behaviors

- `typing_log_round_trip` - a four-entry log (`[100, 250, 500, 1200]`) serializes and deserializes without data loss.

## TypingLog::load

```rust
pub fn load(path: &Path) -> Self
```

Reads and deserializes `typing.json`. Returns an empty `TypingLog` on any error.

### Inputs

- `path: &Path` - file to read. *Why empty on error rather than `Result`:* a missing typing log means smart-zoom-hold simply has no keystroke data to extend holds with; this is the correct degraded behavior, not an error.*

### Returns

`TypingLog` - populated if the file exists and is valid; `TypingLog { ms: vec![] }` otherwise.

### Behaviors

- `typing_log_missing_file_returns_default` - a nonexistent path returns `TypingLog::default()` with an empty `ms` vec.
