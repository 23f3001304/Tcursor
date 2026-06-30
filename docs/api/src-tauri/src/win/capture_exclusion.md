# src-tauri/src/win/capture_exclusion.rs

Single-function module that toggles whether a Win32 window is hidden from screen capture, so the TCursor HUD overlay never appears in the user's own recordings while the editor (the same window, maximized) can be screenshotted. Contains no state and no error detail beyond a boolean success flag.

## set_capture_exclusion

```rust
pub fn set_capture_exclusion(hwnd: isize, exclude: bool) -> bool
```

Sets the Win32 `SetWindowDisplayAffinity` flag on a window: `WDA_EXCLUDEFROMCAPTURE` when `exclude` is true (invisible to any screen-capture process - WGC, BitBlt, DXGI), or `WDA_NONE` when false (visible again). On non-Windows targets, a no-op stub that always returns `false`.

### Inputs

- `hwnd: isize` - the raw `HWND` value cast to `isize`, as returned by Tauri's `hwnd()`. *Why `isize` rather than `HWND`:* keeps the signature free of Windows-SDK types so the module compiles on all targets without conditional imports at the call site.
- `exclude: bool` - true to hide the window from capture (the HUD), false to opt it back in (the editor). *Why a toggle:* the one app window is the HUD (excluded) and, when maximized, the editor (capturable); the frontend flips this on view change.

### Returns

`bool` - `true` if `SetWindowDisplayAffinity` succeeded (`is_ok()` on the `WIN32_ERROR` result); `false` on any failure (insufficient privilege, invalid handle, OS too old) or on non-Windows.

### Implementation

1. (Windows only) Choose `WDA_EXCLUDEFROMCAPTURE` if `exclude` else `WDA_NONE`.
2. Reconstruct `HWND` by casting `hwnd as *mut _` and call `SetWindowDisplayAffinity(HWND(...), affinity)`. *Why unsafe:* a raw Win32 API; Tauri's `hwnd()` is a valid `HWND` from the same process, so the cast is sound.
3. Return `.is_ok()`. (non-Windows) Return `false` unconditionally.

### Used by

- `src-tauri/src/lib.rs` (`run` setup) - excludes the HUD at startup via `set_capture_exclusion(hwnd, true)`, gated by the compile-time `CAPTURE_EXCLUDE` constant (default `true`).
- `src-tauri/src/commands.rs` (`set_capturable`) - the runtime toggle the editor calls to opt the window back into capture.
