# src-tauri/src/export/pipeline/run.rs

Thin Tauri command adapter that launches the export on a background thread and bridges its result to the frontend as Tauri events. Contains no rendering logic - it only wires `exporter::export` to the Tauri event bus.

## run_export

```rust
pub fn run_export(app: AppHandle, folder: String)
```

Spawns a background thread that runs the full export and emits progress/completion/error events to the Tauri frontend.

### Inputs

- `app: AppHandle` - Tauri app handle used to emit events. *Why:* cloned once so both the progress closure and the final outcome can emit independently without a move conflict.
- `folder: String` - absolute path to the project folder. *Why:* `ProjectPaths` is constructed from this string inside the thread so the path is owned by the thread with no lifetime issues.

### Returns

`()` - fire-and-forget; result handling is done by the spawned thread via event emission.

### Implementation

1. Query `win::display::primary_refresh_hz()` and cap to 60; this is the capture fps passed to `exporter::export`.
2. Spawn a detached thread: construct `ProjectPaths` from `folder`; clone `app` for the progress callback.
3. Call `exporter::export` with a closure that emits `"export-progress"` (payload `u8` 0..=100) on each percentage advance.
4. On `Ok(())`: emit `"export-done"` with the folder string.
5. On `Err(e)`: emit `"export-error"` with the error's `to_string()`.

### Behaviors worth knowing

- The thread is detached; if the Tauri window closes while exporting, the thread runs to completion and the emit calls silently fail (`.emit()` returns a dropped `Result`).
- The fps cap of 60 matches `OUT_FPS` in `exporter.rs`, so timeline synthesis never overshoots the encoder's target rate.
