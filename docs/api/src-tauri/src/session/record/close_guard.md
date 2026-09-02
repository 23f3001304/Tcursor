# src-tauri/src/session/record/close_guard.rs

The `CloseRequested` safety net (task-6 brief, ruling R6): if the window tries to close while `Recorder::is_busy()`, `lib.rs`'s `on_window_event` handler calls `prevent_close()` and hands off to `finish_and_close` here instead of ever letting the process exit mid-take.

## finish_and_close

```rust
pub async fn finish_and_close(app: tauri::AppHandle)
```

Finishes the take (if nobody has already claimed the stop) and then closes the main window. Spawned once per close attempt from `lib.rs` (guarded there by a `CLOSING` static so a repeated `CloseRequested` while this is already running doesn't spawn a second one).

### Inputs

- `app: tauri::AppHandle` - resolves `Recorder` (managed state) and, at the end, the `"main"` `WebviewWindow` to close.

### Returns

`()`. Errors from `stop_recording` are intentionally discarded (`let _ =`) - by the time this runs, the project folder is already saved either way (see `recorder_stop.md`: `stop_blocking` writes `sync.json`/the manifest from whatever was recorded even when the video finalize itself errors), and there is no HUD left to show an error message to.

### Implementation

1. `recorder.is_recording()` - true means nobody has claimed the stop yet, so THIS call is the stop: awaits `recorder_stop::stop_recording(app.clone())` directly.
2. Otherwise (a stop is already in flight elsewhere - most likely the HUD's own graceful-close path, `useRecordingFlow.stopForClose`) - spawns a `spawn_blocking` closure that polls `recorder.is_busy()` every 25ms until it clears, instead of racing a second, redundant `stop_recording` call. *Why not just call `stop_recording` again:* `stop_blocking`'s first line is `guard.take().ok_or("not recording")` - a second concurrent call would return that error almost immediately, and this function would then close the window before the FIRST call's finalize (still running on its own blocking thread) has actually written the video/manifest.
3. Resolves `app.get_webview_window("main")` and calls `.close()` - unconditionally, whichever branch ran.

### Notes

**Why this does not depend on the frontend.** This is the fallback for the OS close button, Alt+F4, "Close window" from the taskbar, and a wedged renderer - all of which reach `CloseRequested` with no cooperating JS at all (a hung renderer still lets the OS deliver `WM_CLOSE` to the native window). Depending on the HUD's own JS-side graceful stop for correctness here would reintroduce the exact bug this guard exists to close.

**Accepted trade-off.** A webcam `MediaRecorder`'s last buffered chunk lives only in the renderer and cannot be flushed from here, so it may be missing when THIS path (rather than the HUD's own `stopForClose`) is what performs the stop. The primary artifact - screen, mic, system audio, `sync.json`, the manifest - is what must never be lost, and IS finalized by `stop_recording` regardless of the frontend's state.

### Used by

- `src-tauri/src/lib.rs` - spawned from the `on_window_event` `CloseRequested` handler via `tauri::async_runtime::spawn`.
