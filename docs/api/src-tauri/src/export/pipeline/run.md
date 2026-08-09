# src-tauri/src/export/pipeline/run.rs

Thin Tauri command adapter that launches the export on a background thread and bridges its result to the frontend as Tauri events. Contains no rendering logic - it only wires `exporter::export` to the Tauri event bus.

## run_export

```rust
pub fn run_export(app: AppHandle, folder: String, settings: ExportSettings)
```

Spawns a background thread that runs the full export and emits progress/completion/error events to the Tauri frontend.

### Inputs

- `app: AppHandle` - Tauri app handle used to emit events. *Why:* cloned once so both the progress closure and the final outcome can emit independently without a move conflict.
- `folder: String` - absolute path to the project folder. *Why:* `ProjectPaths` is constructed from this string inside the thread so the path is owned by the thread with no lifetime issues.
- `settings: ExportSettings` (`export::settings::ExportSettings`) - the user's chosen resolution/fps/quality/format, collected by `ExportDialog` and passed straight through from the `export_project` command. *Why passed through rather than resolved here:* `exporter::export` is the single place that turns `ExportSettings` into `Layout`/encoder args, so `run_export` stays a thin event-bridging adapter.

### Returns

`()` - fire-and-forget; result handling is done by the spawned thread via event emission.

### Implementation

1. Spawn a detached thread: construct `ProjectPaths` from `folder`; compute `final_path = paths.folder.join("final.<ext>")` (`<ext>` from `settings.format.extension()`) - the exact path `audio_mux::mux` writes the finished export to; clone `app` for the progress callback; move `settings` in.
2. Call `exporter::export(&paths, settings, ...)` with a closure that emits `"export-progress"` (payload `u8` 0..=100) on each percentage advance AND (Task 39) mirrors the same percent onto the Windows taskbar progress bar via `win::sys::brand_icon::set_export_progress(&app2, Some(p))` - brand flair riding the exact same callback, not a second polling path. `settings` is `Copy` (`ExportSettings` derives it), so `final_path` reading `settings.format` before this call and the call itself consuming `settings` by value don't conflict - the call gets its own copy.
3. Once `exporter::export` returns (either outcome), call `brand_icon::set_export_progress(&app, None)` (Task 39) to clear the taskbar bar - a finished export (success OR error) never leaves a stale progress indicator sitting on the icon.
4. On `Ok(())`: emit `"export-done"` with `final_path` (as a `String`, `to_string_lossy().into_owned()`) - NOT the project folder. The frontend stores this and offers a "Show in folder" button (`revealItemInDir`) without needing to re-derive the output filename/extension itself.
5. On `Err(e)`: emit `"export-error"` with the error's `to_string()`.

### Behaviors worth knowing

- The thread is detached; if the Tauri window closes while exporting, the thread runs to completion and the emit calls silently fail (`.emit()` returns a dropped `Result`).
- The display-refresh query (capped at 60) that used to live here now lives inside `exporter::export` itself (as `capture_fps`), since it also feeds `Fps::Source`'s fallback - `run_export` no longer needs to know about it at all.
