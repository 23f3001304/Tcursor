# src-tauri/src/shell/window.rs

The one seam between a Tauri window and the OS handle the ports speak in. Created in cross-platform Phase 1, Batch C3 (2026-09-15) so that neither `ports/` nor `platform/` ever names a `tauri::WebviewWindow`.

## handle

```rust
pub fn handle(window: &tauri::WebviewWindow) -> Option<WindowHandle>
```

Resolves a Tauri window to the opaque `WindowHandle` `SystemPort::exclude_from_capture` takes.

### Inputs

- `window: &tauri::WebviewWindow` - the window to resolve, ALWAYS A PARAMETER. *Why never a lookup inside:* both of today's callers reach for `get_webview_window("main")`, and Studio is a mode inside this same binary with its own launcher window that needs the same capture-exclusion treatment. A helper that picked the window would have to be rewritten the day Studio ships; one that takes it works unchanged.

### Returns

`Option<WindowHandle>` - the raw HWND as an `isize` on Windows, `None` when Tauri cannot produce a handle and on every non-Windows target. *Why `Option` rather than a zero handle:* the callers already have a "we could not do it" path (both return or log `false`), and a zero HWND would be passed to Win32 as a valid-looking argument.

### Implementation

1. (Windows) `window.hwnd().ok().map(|h| WindowHandle(h.0 as isize))`.
2. (non-Windows) `None`.

### Why it is in `shell/` and not in `ports/` or `platform/`

`shell/` is where Tauri types are allowed. The recording core keeps them out on purpose (`Notify` is an `Arc<dyn Fn(&str)>` for the same reason), so the conversion happens once, here, at the composition boundary, and the port sees only an `isize`.

### Used by

- `src-tauri/src/lib.rs` (`run` setup) - resolves the main window before excluding it from capture at startup.
- `src-tauri/src/commands.rs` (`set_capturable`) - resolves the main window for the runtime toggle.
